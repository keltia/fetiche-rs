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
use ractor::{call, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::trace;

use crate::actors::{QueueMsg, StatsMsg};
use crate::Stats;

/// Messages that can be handled by the Runner actor.
///
/// This enum defines the possible messages that control the Runner actor's behavior:
/// - `Start`: Initiates execution of a job with the given ID
/// - `Stop`: Terminates execution of a job with the given ID
/// - `Stats`: Retrieves current execution statistics
///
#[derive(Debug)]
pub enum RunnerMsg {
    /// Start executing the job with the specified ID
    Start(usize),
    /// Stop executing the job with the specified ID
    Stop(usize),
    /// Request current execution statistics
    Stats(RpcReplyPort<Stats>),
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
            RunnerMsg::Start(n) => {
                let queue = state.queue.clone();
                let mut job = call!(queue, |port| QueueMsg::GetById(n, port)).unwrap();

                let mut data = vec![];
                let _ = job.run(&mut data)?;
            }
            RunnerMsg::Stop(n) => {
                todo!()
            }
            RunnerMsg::Stats(sender) => {
                todo!()
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
        (RunnerActor, RunnerArgs { queue: self.queue.clone(), stats: self.stat.clone() })
    }
}
