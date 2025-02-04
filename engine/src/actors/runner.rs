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

use ractor::factory::{FactoryMessage, Job, Worker, WorkerBuilder, WorkerId};
use ractor::{call, cast, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::trace;

use crate::actors::{QueueMsg, StatsMsg};
use crate::{JobState, Stats};

/// Messages that can be handled by the Runner actor.
///
/// This enum defines the possible messages that control the Runner actor's behavior:
/// - `Run`: Initiates execution of next job in the queue
///
#[derive(Debug)]
pub enum RunnerMsg {
    /// Run next job
    Run(RpcReplyPort<Stats>),
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
    /// Reference to the Queue actor for queue management
    pub queue: ActorRef<QueueMsg>,
    /// Reference to the Stats actor for metrics
    pub stats: ActorRef<StatsMsg>,
}

#[ractor::async_trait]
impl Worker for RunnerActor {
    type Key = usize;
    type Message = RunnerMsg;
    type Arguments = RunnerArgs;
    type State = RunnerArgs;

    #[tracing::instrument(skip(self, _factory))]
    async fn pre_start(
        &self,
        _wid: WorkerId,
        _factory: &ActorRef<FactoryMessage<usize, RunnerMsg>>,
        startup_context: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(startup_context)
    }

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
            RunnerMsg::Run(sender) => {
                let stat = state.stats.clone();
                let queue = state.queue.clone();

                let mut job = call!(queue, |port| QueueMsg::Run(port))?;
                job.stats(stat.clone());
                job.state = JobState::Running;

                let job_tag = format!("job#{}", job.id);
                let _ = cast!(stat, StatsMsg::New(job_tag.clone()))?;

                let mut data = vec![];
                let _ = job.run(&mut data)?;

                let stats = call!(stat, |port| StatsMsg::Get(job_tag.clone(), port))?;
                let _ = sender.send(stats);
                let _ = cast!(stat, StatsMsg::Reset(job_tag))?;

                job.state = JobState::Completed;
            }
        }
        Ok(key)
    }
}

#[derive(Clone, Debug)]
pub struct RunnerBuilder {
    /// Reference to the Queue actor for queue management
    pub queue: ActorRef<QueueMsg>,
    /// Reference to the Stats actor for metrics
    pub stat: ActorRef<StatsMsg>,
}

impl WorkerBuilder<RunnerActor, RunnerArgs> for RunnerBuilder {
    #[tracing::instrument(skip(self))]
    fn build(&mut self, _wid: WorkerId) -> (RunnerActor, RunnerArgs) {
        (
            RunnerActor,
            RunnerArgs {
                queue: self.queue.clone(),
                stats: self.stat.clone(),
            },
        )
    }
}
