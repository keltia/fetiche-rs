//! The runner is a Ractor factory worker that processes dispatched jobs.
//!
//! It registers per-job statistics, supplies engine dependencies to the job,
//! and delegates stage execution to [`run_pipeline`]. The pipeline runs the
//! stages concurrently and returns their terminal status. The runner collects
//! the statistics snapshot, submits the outcome to the results actor, and
//! then notifies the caller waiting for the job.
//!
//! # Components
//!
//! - [`RunnerMsg`] carries a dispatched job and its acknowledgement port.
//! - [`RunnerActor`] implements the factory worker.
//! - [`RunnerArgs`] supplies the channel capacity, cancellation signal, and
//!   actor handles shared with each worker.

use ractor::factory::{FactoryMessage, Job, Worker, WorkerBuilder, WorkerId};
use ractor::{ActorProcessingErr, ActorRef, RpcReplyPort, call, cast};
use tokio::sync::watch;
use tracing::{debug, info, trace};

use crate::actors::{ResultsMsg, StatsMsg};
use crate::pipeline::run_pipeline;
use crate::{JobOutcome, JobStatus, Stats, Work};
#[cfg(feature = "asd")]
use crate::{Producer, TokenMsg};

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
/// The stats actor tracks per-job metrics, and the results actor stores each
/// terminal outcome. With the ASD feature enabled, the token actor is passed
/// to fetch stages. The worker also passes the channel capacity and engine
/// cancellation signal to the pipeline.
///
#[derive(Clone, Debug)]
pub struct RunnerArgs {
    pub queue_capacity: usize,
    pub cancel: watch::Receiver<bool>,
    /// Engine token actor used by fetch stages.
    #[cfg(feature = "asd")]
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
    #[cfg(feature = "asd")]
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
            #[cfg(feature = "asd")]
            tokens: startup_context.tokens,
            results: startup_context.results.clone(),
            stats: startup_context.stats.clone(),
        };
        Ok(state)
    }

    /// Runs a dispatched job, collects metrics, and publishes its outcome before notifying the waiter.
    ///
    /// The worker initializes metrics and delegates execution to [`run_pipeline`].
    /// It then reads and resets the job's metrics and submits the status and
    /// statistics to the results actor. The job waiter receives its outcome
    /// after that submission attempt. The worker acknowledges dispatch separately.
    ///
    /// `wid` identifies the worker. The dispatched message carries the job's
    /// factory key and its work envelope. `state` holds the actor handles and
    /// pipeline settings. The worker returns the factory key after handling
    /// the dispatch, including when the job itself fails.
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
                        #[cfg(feature = "asd")]
                        if let Producer::Fetch(fetch) = &mut job.producer {
                            fetch.tokens = state.tokens.clone();
                        }
                        outcome.status =
                            run_pipeline(job, state.queue_capacity, state.cancel.clone()).await;
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

#[derive(Clone, Debug)]
pub struct RunnerBuilder {
    pub queue_capacity: usize,
    pub cancel: watch::Receiver<bool>,
    /// Engine token actor used by fetch stages.
    #[cfg(feature = "asd")]
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
                #[cfg(feature = "asd")]
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
    use crate::{Consumer, Job as EngineJob, Producer, Read, Stdout};
    use ractor::Actor;
    use ractor::factory::{Factory, FactoryArguments, JobOptions, queues, routing};
    use std::time::Duration;
    use tokio::time::timeout;

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
                #[cfg(feature = "asd")]
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
}
