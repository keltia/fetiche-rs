//! This module implements the Runner actor responsible for executing jobs in the engine.
//!
//! The Runner actor is a worker that processes jobs from a queue and manages their execution.
//! It maintains communication with the queue actor for job retrieval and reports statistics
//! about job execution.
//!
//! # Components
//!
//! - `RunnerMsg`: Enum defining the messages that can be sent to the Runner actor
//! - `RunnerActor`: The actual actor implementation that processes jobs
//! - `RunnerArgs`: Configuration and dependencies required by the Runner actor
//!
use futures::stream::{FuturesUnordered, StreamExt};

use ractor::factory::{FactoryMessage, Job, Worker, WorkerBuilder, WorkerId};
use ractor::{ActorProcessingErr, ActorRef, RpcReplyPort, call, cast};
use tokio::sync::mpsc::{Receiver, channel};
use tokio::sync::watch;
use tokio::task::{AbortHandle, JoinHandle};
use tracing::{debug, info, trace};

use crate::task::Runnable;

use crate::actors::{ResultsMsg, StatsMsg, TokenMsg};
use crate::{
    Consumer, Job as EngineJob, JobOutcome, JobStatus, Middle, Producer, Stats, Task, Work,
};

/// Messages that can be handled by the Runner actor.
///
/// This enum defines the possible messages that control the Runner actor's behavior:
/// - `Run`: Initiates execution of next job in the queue
///
#[derive(Debug)]
pub enum RunnerMsg {
    /// Run next job
    Run(Work, RpcReplyPort<()>),
}

/// The Runner actor implementation that processes jobs from the queue.
///
/// This actor is responsible for the actual execution of jobs, managing their
/// lifecycle, and reporting execution statistics.
#[derive(Debug, Default)]
pub struct RunnerActor;

/// Configuration and dependencies required by the Runner actor.
///
/// Contains references to other actors that the Runner needs to interact with:
/// - queue: Reference to the Queue actor for retrieving jobs
/// - stats: Reference to the Stats actor for reporting metrics
///
#[derive(Clone, Debug)]
pub struct RunnerArgs {
    pub queue_capacity: usize,
    pub cancel: watch::Receiver<bool>,
    /// Engine token actor used by fetch stages.
    pub tokens: Option<ActorRef<TokenMsg>>,
    /// Reference to the Result actor
    pub results: ActorRef<ResultsMsg>,
    /// Reference to the Stats actor for metrics
    pub stats: ActorRef<StatsMsg>,
}

#[derive(Clone, Debug)]
pub struct RunnerState {
    queue_capacity: usize,
    cancel: watch::Receiver<bool>,
    tokens: Option<ActorRef<TokenMsg>>,
    /// Reference to the Result actor
    pub results: ActorRef<ResultsMsg>,
    /// Reference to the Stats actor for metrics
    pub stats: ActorRef<StatsMsg>,
}

#[ractor::async_trait]
impl Worker for RunnerActor {
    type Key = usize;
    type Message = RunnerMsg;
    type Arguments = RunnerArgs;
    type State = RunnerState;

    /// Attaches the engine's cancellation signal to the worker's state.
    #[tracing::instrument(skip(self, _factory))]
    async fn pre_start(
        &self,
        wid: WorkerId,
        _factory: &ActorRef<FactoryMessage<usize, RunnerMsg>>,
        startup_context: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        trace!("runner {} starting", wid);
        let state = RunnerState {
            queue_capacity: startup_context.queue_capacity,
            cancel: startup_context.cancel,
            tokens: startup_context.tokens,
            results: startup_context.results.clone(),
            stats: startup_context.stats.clone(),
        };
        Ok(state)
    }

    /// Handles incoming messages for the Runner actor.
    ///
    /// This method processes RunnerMsg messages to execute jobs in a pipeline pattern:
    /// - Creates a pipeline of tasks from producer through filters to consumer
    /// - Executes tasks sequentially, passing data between them
    /// - Collects execution statistics and reports results
    ///
    /// # Parameters
    /// * `wid` - Worker ID for this Runner instance
    /// * `_factory` - Reference to the actor factory (unused)
    /// * `msg` - The message containing job details to be processed
    /// * `key` - Unique identifier for this job execution
    /// * `state` - Mutable reference to actor's state containing results/stats actors
    ///
    /// # Returns
    /// Returns the job key on successful execution, or an ActorProcessingErr on failure
    ///
    #[tracing::instrument(skip_all)]
    async fn handle(
        &self,
        wid: WorkerId,
        _factory: &ActorRef<FactoryMessage<usize, RunnerMsg>>,
        Job { msg, key, .. }: Job<usize, RunnerMsg>,
        state: &mut Self::State,
    ) -> Result<usize, ActorProcessingErr> {
        trace!(worker_id = wid, job_id = key, "Runner received job");
        match msg {
            // Takes the next job from the queue.
            //
            RunnerMsg::Run(work, sender) => {
                let stat = state.stats.clone();
                let results = state.results.clone();

                let Work {
                    mut job,
                    completion,
                } = work;
                job.register(stat.clone());
                info!(job_id = job.id, "Job({wid})::run({})", job.name);

                let id = job.id;
                let job_tag = format!("job#{id}");
                let mut outcome = JobOutcome {
                    id,
                    status: JobStatus::Succeeded,
                    stats: Stats::default(),
                };

                match cast!(stat, StatsMsg::New(job_tag.clone())) {
                    Ok(_) => {
                        outcome.status = run_pipeline(
                            job,
                            state.queue_capacity,
                            state.cancel.clone(),
                            state.tokens.clone(),
                        )
                        .await;
                        match call!(stat, |port| StatsMsg::Get(job_tag.clone(), port)) {
                            Ok(snapshot) => outcome.stats = snapshot,
                            Err(err) if outcome.status == JobStatus::Succeeded => {
                                outcome.status = job_failure(format!("read statistics: {err}"));
                            }
                            Err(_) => debug!(job_id = id, "Could not read job statistics"),
                        }
                        let _ = cast!(stat, StatsMsg::Reset(job_tag));
                    }
                    Err(err) => {
                        outcome.status = job_failure(format!("initialize statistics: {err}"))
                    }
                }

                if let Err(err) = call!(results, |port| ResultsMsg::Submit(outcome.clone(), port)) {
                    if outcome.status == JobStatus::Succeeded {
                        outcome.status = job_failure(format!("publish outcome: {err}"));
                    }
                    debug!(job_id = id, "Could not publish job outcome");
                }

                completion.send(outcome);
                let _ = sender.send(());
            }
        }
        Ok(key)
    }
}

/// Records a failure in job coordination without a stage identifier.
fn job_failure(error: String) -> JobStatus {
    JobStatus::Failed { stage: None, error }
}

/// Records a failure with the stage label used in the job outcome.
fn stage_failure(stage: String, error: String) -> JobStatus {
    JobStatus::Failed {
        stage: Some(stage),
        error,
    }
}

/// Validates, wires, and starts the producer, middle stages, and consumer.
///
/// The runner sends a single trigger to the producer, drains and discards terminal
/// messages, and waits for every stage. An invalid stage fails before any tasks
/// start. An execution failure or cancellation causes remaining tasks to be
/// aborted and joined before the status is returned. The engine can cancel all
/// stages through `cancel` when its shutdown grace period expires.
///
/// Each channel has `queue_capacity` message slots. The caller must supply a
/// capacity accepted by Tokio MPSC.
/// Engine workers pass their token actor to ASD fetch stages.
async fn run_pipeline(
    mut job: EngineJob,
    queue_capacity: usize,
    cancel: watch::Receiver<bool>,
    tokens: Option<ActorRef<TokenMsg>>,
) -> JobStatus {
    if *cancel.borrow() {
        return JobStatus::Cancelled { stage: None };
    }
    #[cfg(feature = "asd")]
    if let Producer::Fetch(fetch) = &mut job.producer {
        fetch.tokens = tokens;
    }
    #[cfg(not(feature = "asd"))]
    let _ = (&mut job, tokens);
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
        return job_failure("could not start producer: receiver closed".into());
    }
    status
}

/// Owns the stage joins and retains abort handles if the runner is dropped.
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

#[derive(Clone, Debug)]
pub struct RunnerBuilder {
    pub queue_capacity: usize,
    pub cancel: watch::Receiver<bool>,
    /// Engine token actor used by fetch stages.
    pub tokens: Option<ActorRef<TokenMsg>>,
    /// Reference to the Results actor
    pub results: ActorRef<ResultsMsg>,
    /// Reference to the Stats actor for metrics
    pub stat: ActorRef<StatsMsg>,
}

impl WorkerBuilder<RunnerActor, RunnerArgs> for RunnerBuilder {
    /// Builds a worker with the shared engine cancellation receiver.
    #[tracing::instrument(skip(self))]
    fn build(&mut self, wid: WorkerId) -> (RunnerActor, RunnerArgs) {
        (
            RunnerActor,
            RunnerArgs {
                queue_capacity: self.queue_capacity,
                cancel: self.cancel.clone(),
                tokens: self.tokens.clone(),
                results: self.results.clone(),
                stats: self.stat.clone(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::{ResultsActor, StatsActor};
    use crate::{Dummy, Read, Stdout, Tee};
    use ractor::Actor;
    use ractor::factory::{Factory, FactoryArguments, JobOptions, queues, routing};
    use std::time::Duration;
    use tokio::time::timeout;

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

    /// Exercises runner dispatch and checks that the outcome is already retrievable on notification.
    #[tokio::test]
    async fn failed_outcome_is_published_before_the_waiter_is_notified() {
        let (stats, _) = Actor::spawn(Some("runner-test-stats".into()), StatsActor, "test".into())
            .await
            .unwrap();
        let (results, _) = Actor::spawn(None, ResultsActor, ()).await.unwrap();
        let factory = Factory::<
            usize,
            RunnerMsg,
            RunnerArgs,
            RunnerActor,
            routing::QueuerRouting<usize, RunnerMsg>,
            queues::DefaultQueue<usize, RunnerMsg>,
        >::default();
        let args = FactoryArguments::builder()
            .worker_builder(Box::new(RunnerBuilder {
                queue_capacity: 1,
                cancel: tokio::sync::watch::channel(false).1,
                tokens: None,
                results: results.clone(),
                stat: stats.clone(),
            }))
            .queue(Default::default())
            .router(Default::default())
            .num_initial_workers(1)
            .build();
        let (factory, _) = Actor::spawn(None, factory, args).await.unwrap();
        let job = EngineJob::builder()
            .id(7)
            .producer(Producer::Read(Read::new("uninitialized")))
            .consumer(Consumer::Stdout(Stdout::new()))
            .build();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let work = Work::new(job, tx);
        let (dispatch_reply, _) = tokio::sync::oneshot::channel();
        factory
            .cast(FactoryMessage::Dispatch(Job {
                key: 7,
                msg: RunnerMsg::Run(work, dispatch_reply.into()),
                options: JobOptions::default(),
                accepted: None,
            }))
            .unwrap();

        let outcome = timeout(Duration::from_secs(5), rx).await.unwrap().unwrap();
        assert!(matches!(outcome.status, JobStatus::Failed { .. }));
        let stored = call!(results, |port| ResultsMsg::Fetch(7, port)).unwrap();
        assert_eq!(stored, Some(outcome));

        factory.stop(None);
        stats.stop(None);
        results.stop(None);
    }

    /// Runs a producer, a tee, and a sink, then checks the tee's output after completion.
    #[tokio::test]
    async fn successful_pipeline_waits_for_its_side_effects() {
        let directory = tempfile::tempdir().unwrap();
        let copy = directory.path().join("copy.txt");
        let mut job = EngineJob::builder()
            .id(1)
            .producer(Producer::Dummy(Dummy::new()))
            .consumer(Consumer::Stdout(Stdout::new()))
            .build();
        job.add(Middle::Tee(Tee::into(copy.to_str().unwrap())));

        assert_eq!(
            timeout(
                Duration::from_secs(5),
                run_pipeline(job, 1, watch::channel(false).1, None)
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
        let job = EngineJob::builder()
            .id(1)
            .producer(Producer::Read(Read::new("uninitialized")))
            .consumer(Consumer::Stdout(Stdout::new()))
            .build();

        let status = timeout(
            Duration::from_secs(5),
            run_pipeline(job, 1, watch::channel(false).1, None),
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
