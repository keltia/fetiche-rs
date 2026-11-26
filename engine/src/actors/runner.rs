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
use tokio::task::{AbortHandle, JoinHandle};
use tracing::{debug, info, trace};

use crate::task::Runnable;

use crate::actors::{ResultsMsg, StatsMsg};
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
    /// Reference to the Result actor
    pub results: ActorRef<ResultsMsg>,
    /// Reference to the Stats actor for metrics
    pub stats: ActorRef<StatsMsg>,
}

#[derive(Clone, Debug)]
pub struct RunnerState {
    queue_capacity: usize,
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
    #[tracing::instrument(skip(self, _factory))]
    async fn handle(
        &self,
        wid: WorkerId,
        _factory: &ActorRef<FactoryMessage<usize, RunnerMsg>>,
        Job { msg, key, .. }: Job<usize, RunnerMsg>,
        state: &mut Self::State,
    ) -> Result<usize, ActorProcessingErr> {
        trace!("runner {} got message: {:?}", wid, msg);
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
                        outcome.status = run_pipeline(job, state.queue_capacity).await;
                        match call!(stat, |port| StatsMsg::Get(job_tag.clone(), port)) {
                            Ok(snapshot) => outcome.stats = snapshot,
                            Err(err) if outcome.status == JobStatus::Succeeded => {
                                outcome.status = job_failure(format!("read statistics: {err}"));
                            }
                            Err(err) => debug!(job_id = id, %err, "Could not read job statistics"),
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
                    debug!(job_id = id, %err, "Could not publish job outcome");
                }

                completion.send(outcome);
                let _ = sender.send(());
            }
        }
        Ok(key)
    }
}

fn job_failure(error: String) -> JobStatus {
    JobStatus::Failed { stage: None, error }
}

fn stage_failure(stage: String, error: String) -> JobStatus {
    JobStatus::Failed {
        stage: Some(stage),
        error,
    }
}

async fn run_pipeline(job: EngineJob, queue_capacity: usize) -> JobStatus {
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

    let status = tasks.drain(output).await;
    if status == JobStatus::Succeeded && started.is_err() {
        return job_failure("could not start producer: receiver closed".into());
    }
    status
}

#[derive(Default)]
struct StageTasks {
    pending: FuturesUnordered<StageJoin>,
    abort_handles: Vec<AbortHandle>,
}

type StageJoin =
    futures::future::BoxFuture<'static, (String, Result<eyre::Result<()>, tokio::task::JoinError>)>;

impl StageTasks {
    fn push(&mut self, name: String, handle: JoinHandle<eyre::Result<()>>) {
        self.abort_handles.push(handle.abort_handle());
        self.pending
            .push(Box::pin(async move { (name, handle.await) }));
    }

    async fn drain(mut self, mut output: Receiver<String>) -> JobStatus {
        let mut output_open = true;
        while output_open || !self.pending.is_empty() {
            tokio::select! {
                message = output.recv(), if output_open => {
                    output_open = message.is_some();
                }
                joined = self.pending.next(), if !self.pending.is_empty() => {
                    let Some((stage, result)) = joined else { continue };
                    let status = match result {
                        Ok(Ok(())) => continue,
                        Ok(Err(err)) => stage_failure(stage, format!("{err:#}")),
                        Err(err) if err.is_cancelled() => JobStatus::Cancelled { stage: Some(stage) },
                        Err(err) => stage_failure(stage, err.to_string()),
                    };
                    drop(output);
                    self.abort_all();
                    while self.pending.next().await.is_some() {}
                    return status;
                }
            }
        }
        JobStatus::Succeeded
    }

    fn abort_all(&self) {
        for handle in &self.abort_handles {
            handle.abort();
        }
    }
}

impl Drop for StageTasks {
    fn drop(&mut self) {
        self.abort_all();
    }
}

#[derive(Clone, Debug)]
pub struct RunnerBuilder {
    pub queue_capacity: usize,
    /// Reference to the Results actor
    pub results: ActorRef<ResultsMsg>,
    /// Reference to the Stats actor for metrics
    pub stat: ActorRef<StatsMsg>,
}

impl WorkerBuilder<RunnerActor, RunnerArgs> for RunnerBuilder {
    #[tracing::instrument(skip(self))]
    fn build(&mut self, wid: WorkerId) -> (RunnerActor, RunnerArgs) {
        (
            RunnerActor,
            RunnerArgs {
                queue_capacity: self.queue_capacity,
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
            timeout(Duration::from_secs(5), run_pipeline(job, 1))
                .await
                .unwrap(),
            JobStatus::Succeeded
        );
        assert_eq!(std::fs::read_to_string(copy).unwrap(), "DUMMY");
    }

    #[tokio::test]
    async fn stage_execute_error_is_reported_with_its_name() {
        let job = EngineJob::builder()
            .id(1)
            .producer(Producer::Read(Read::new("uninitialized")))
            .consumer(Consumer::Stdout(Stdout::new()))
            .build();

        let status = timeout(Duration::from_secs(5), run_pipeline(job, 1))
            .await
            .unwrap();
        assert!(matches!(
            status,
            JobStatus::Failed { stage: Some(stage), .. } if stage == "1 (Read)"
        ));
    }

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

        let status = timeout(Duration::from_secs(5), stages.drain(output_rx))
            .await
            .unwrap();
        assert!(matches!(
            status,
            JobStatus::Failed { stage: Some(stage), error }
                if stage == "panicking" && error.contains("test panic")
        ));
    }
}
