//! Stage wiring, execution, and supervision for a single job.

use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::mpsc::{Receiver, channel};
use tokio::sync::watch;
use tokio::task::{AbortHandle, JoinHandle};

use crate::{Consumer, Job, JobStatus, Middle, Producer, Runnable, Task};

/// Records a failure with the stage label used in the job outcome.
fn stage_failure(stage: String, error: String) -> JobStatus {
    JobStatus::Failed {
        stage: Some(stage),
        error,
    }
}

/// Validates, wires, and starts the stages of a job prepared by the caller.
///
/// The pipeline sends a single trigger to the producer, drains and discards terminal
/// messages, and waits for every stage. An invalid stage fails before any tasks
/// start. An execution failure or cancellation causes remaining tasks to be
/// aborted and joined before the status is returned. The engine can cancel all
/// stages through `cancel` when its shutdown grace period expires.
///
/// Each channel has `queue_capacity` message slots. The caller must supply a
/// capacity accepted by Tokio MPSC.
pub(crate) async fn run_pipeline(
    job: Job,
    queue_capacity: usize,
    cancel: watch::Receiver<bool>,
) -> JobStatus {
    if *cancel.borrow() {
        return JobStatus::Cancelled { stage: None };
    }
    let mut stages = Vec::with_capacity(job.middle.len() + 2);
    let mut add_stage = |name: String, task, invalid| {
        stages.push((format!("{} ({name})", stages.len() + 1), task, invalid));
    };
    let invalid = matches!(job.producer, Producer::Invalid);
    add_stage(job.producer.to_string(), Task::from(job.producer), invalid);
    for middle in job.middle {
        let invalid = matches!(middle, Middle::Invalid);
        add_stage(middle.to_string(), Task::from(middle), invalid);
    }
    let invalid = matches!(job.consumer, Consumer::Invalid);
    add_stage(job.consumer.to_string(), Task::from(job.consumer), invalid);

    if let Some((name, _, _)) = stages.iter().find(|(_, _, invalid)| *invalid) {
        return stage_failure(name.clone(), "invalid stage".into());
    }

    let (trigger, mut output) = channel(queue_capacity);
    let mut tasks = StageTasks::default();
    for (name, mut stage, _) in stages {
        let (next, handle) = stage.run(output).await;
        tasks.push(name, handle);
        output = next;
    }
    let started = trigger.send("start".to_string()).await;
    drop(trigger);

    let status = tasks.drain(output, cancel).await;
    if status == JobStatus::Succeeded && started.is_err() {
        return JobStatus::Failed {
            stage: None,
            error: "could not start producer: receiver closed".into(),
        };
    }
    status
}

/// Owns the stage joins and retains abort handles if the pipeline future is dropped.
#[derive(Default)]
struct StageTasks {
    pending: FuturesUnordered<StageJoin>,
    abort_handles: Vec<AbortHandle>,
}

type StageJoin =
    futures::future::BoxFuture<'static, (String, Result<eyre::Result<()>, tokio::task::JoinError>)>;

impl StageTasks {
    /// Tracks a running stage and the label to report if its join fails.
    fn push(&mut self, name: String, handle: JoinHandle<eyre::Result<()>>) {
        self.abort_handles.push(handle.abort_handle());
        self.pending
            .push(Box::pin(async move { (name, handle.await) }));
    }

    /// Consumes terminal messages while polling stage joins so a full output
    /// channel cannot prevent a stage from finishing.
    ///
    /// The first failed join or cancellation signal determines the status. The
    /// receiver is dropped, and remaining tasks are aborted and joined.
    async fn drain(
        mut self,
        mut output: Receiver<String>,
        mut cancel: watch::Receiver<bool>,
    ) -> JobStatus {
        let mut output_open = true;
        let status = loop {
            if !output_open && self.pending.is_empty() {
                return JobStatus::Succeeded;
            }
            tokio::select! {
                true = async { cancel.wait_for(|requested| *requested).await.is_ok() } => {
                    break JobStatus::Cancelled { stage: None };
                }
                message = output.recv(), if output_open => {
                    output_open = message.is_some();
                }
                joined = self.pending.next(), if !self.pending.is_empty() => {
                    let Some((stage, result)) = joined else { continue };
                    break match result {
                        Ok(Ok(())) => continue,
                        Ok(Err(err)) => stage_failure(stage, format!("{err:#}")),
                        Err(err) if err.is_cancelled() => JobStatus::Cancelled { stage: Some(stage) },
                        Err(err) => stage_failure(stage, err.to_string()),
                    };
                }
            }
        };
        drop(output);
        self.abort_all();
        while self.pending.next().await.is_some() {}
        status
    }

    /// Requests cancellation for every registered task, including unfinished joins.
    fn abort_all(&self) {
        for handle in &self.abort_handles {
            handle.abort();
        }
    }
}

impl Drop for StageTasks {
    /// Requests cancellation if supervision ends before all tasks are joined.
    fn drop(&mut self) {
        self.abort_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dummy, Read, Stdout, Tee};
    use std::time::Duration;
    use tokio::time::timeout;

    /// Reports the invalid middle stage before the producer can execute.
    #[tokio::test]
    async fn invalid_middle_stage_is_reported_before_execution() {
        let mut job = Job::builder()
            .id(1)
            .producer(Producer::Read(Read::new("uninitialized")))
            .consumer(Consumer::Stdout(Stdout::new()))
            .build();
        job.add(Middle::Invalid);

        let status = run_pipeline(job, 1, watch::channel(false).1).await;
        assert_eq!(
            status,
            JobStatus::Failed {
                stage: Some("2 (Invalid)".into()),
                error: "invalid stage".into(),
            }
        );
    }

    /// A cancellation already requested at entry takes precedence over stage validation.
    #[tokio::test]
    async fn pending_cancellation_skips_pipeline_validation() {
        let job = Job::builder().id(1).build();
        let (_cancel, signal) = watch::channel(true);
        assert_eq!(
            run_pipeline(job, 1, signal).await,
            JobStatus::Cancelled { stage: None }
        );
    }

    /// Checks that a capacity-one edge stalls its sending stage and preserves message order.
    #[tokio::test]
    async fn stage_waits_for_capacity_and_preserves_order() {
        let (input, receiver) = channel(1);
        let (mut output, task) = crate::Copy::new().run(receiver).await;
        assert_eq!(output.max_capacity(), 1);
        timeout(Duration::from_secs(5), async {
            input.send("first".into()).await.unwrap();
            input.send("second".into()).await.unwrap();
            // This permit establishes that the stage has taken the second message.
            drop(input.reserve().await.unwrap());
            drop(input);

            assert_eq!(output.len(), 1);
            assert!(!task.is_finished());
            assert_eq!(output.recv().await.as_deref(), Some("first"));
            assert_eq!(output.recv().await.as_deref(), Some("second"));
            assert!(output.recv().await.is_none());
            task.await.unwrap().unwrap();
        })
        .await
        .unwrap();
    }

    /// Runs a producer, a tee, and a sink, then checks the tee's output after completion.
    #[tokio::test]
    async fn successful_pipeline_waits_for_its_side_effects() {
        let directory = tempfile::tempdir().unwrap();
        let copy = directory.path().join("copy.txt");
        let mut job = Job::builder()
            .id(1)
            .producer(Producer::Dummy(Dummy::new()))
            .consumer(Consumer::Stdout(Stdout::new()))
            .build();
        job.add(Middle::Tee(Tee::into(copy.to_str().unwrap())));

        assert_eq!(
            timeout(
                Duration::from_secs(5),
                run_pipeline(job, 1, watch::channel(false).1)
            )
            .await
            .unwrap(),
            JobStatus::Succeeded
        );
        assert_eq!(std::fs::read_to_string(copy).unwrap(), "DUMMY");
    }

    /// Checks that a producer error identifies its position and stage type.
    #[tokio::test]
    async fn stage_execute_error_is_reported_with_its_name() {
        let job = Job::builder()
            .id(1)
            .producer(Producer::Read(Read::new("uninitialized")))
            .consumer(Consumer::Stdout(Stdout::new()))
            .build();

        let status = timeout(
            Duration::from_secs(5),
            run_pipeline(job, 1, watch::channel(false).1),
        )
        .await
        .unwrap();
        assert!(matches!(
            status,
            JobStatus::Failed { stage: Some(stage), .. } if stage == "1 (Read)"
        ));
    }

    /// Checks that a panic stops an idle peer with a live output sender.
    #[tokio::test]
    async fn task_panic_cancels_and_joins_a_pending_stage() {
        let mut stages = StageTasks::default();
        let (output_tx, output_rx) = channel(1);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        stages.push(
            "pending".into(),
            tokio::spawn(async move {
                let _ = ready_tx.send(());
                std::future::pending::<()>().await;
                drop(output_tx);
                Ok(())
            }),
        );
        stages.push(
            "panicking".into(),
            tokio::spawn(async move {
                ready_rx.await?;
                panic!("test panic");
            }),
        );

        let status = timeout(
            Duration::from_secs(5),
            stages.drain(output_rx, watch::channel(false).1),
        )
        .await
        .unwrap();
        assert!(matches!(
            status,
            JobStatus::Failed { stage: Some(stage), error }
                if stage == "panicking" && error.contains("test panic")
        ));
    }

    /// Confirms an aborted stage produces a typed cancellation carrying its name.
    #[tokio::test]
    async fn aborted_stage_reports_cancelled_status() {
        let mut stages = StageTasks::default();
        let (output_tx, output_rx) = channel(1);
        let task = tokio::spawn(std::future::pending::<eyre::Result<()>>());
        task.abort();
        stages.push("aborted".into(), task);

        // Keep the terminal channel open to ensure cancellation can finish without EOF.
        let status = timeout(
            Duration::from_secs(5),
            stages.drain(output_rx, watch::channel(false).1),
        )
        .await
        .unwrap();
        drop(output_tx);
        assert_eq!(
            status,
            JobStatus::Cancelled {
                stage: Some("aborted".into())
            }
        );
    }

    /// The engine cancellation signal joins a pending stage before returning.
    #[tokio::test]
    async fn shutdown_signal_cancels_and_joins_pipeline_stages() {
        let mut stages = StageTasks::default();
        let (output_tx, output_rx) = channel(1);
        let (cancel, signal) = watch::channel(false);
        let (started, ready) = tokio::sync::oneshot::channel();
        let (stage_alive, mut stage_closed) = tokio::sync::oneshot::channel::<()>();
        stages.push(
            "stream".into(),
            tokio::spawn(async move {
                let _ = started.send(());
                std::future::pending::<()>().await;
                drop(stage_alive);
                drop(output_tx);
                Ok(())
            }),
        );
        ready.await.unwrap();
        cancel.send_replace(true);
        assert_eq!(
            timeout(Duration::from_secs(5), stages.drain(output_rx, signal))
                .await
                .unwrap(),
            JobStatus::Cancelled { stage: None }
        );
        assert!(matches!(
            stage_closed.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Closed)
        ));
    }
}
