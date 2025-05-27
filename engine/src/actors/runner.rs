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
use std::io::Write;

use futures::stream::{self, StreamExt};
use ractor::factory::{FactoryMessage, Job, Worker, WorkerBuilder, WorkerId};
use ractor::{call, cast, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::{debug, info, trace};

use crate::task::Runnable;

use crate::actors::{ResultsMsg, StatsMsg};
use crate::{Stats, Task, Work};

/// Messages that can be handled by the Runner actor.
///
/// This enum defines the possible messages that control the Runner actor's behavior:
/// - `Run`: Initiates execution of next job in the queue
///
#[derive(Debug)]
pub enum RunnerMsg {
    /// Run next job
    Run(Work, RpcReplyPort<Stats>),
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
    /// Reference to the Result actor
    pub results: ActorRef<ResultsMsg>,
    /// Reference to the Stats actor for metrics
    pub stats: ActorRef<StatsMsg>,
}

#[derive(Clone, Debug)]
pub struct RunnerState {
    /// ID of the runner.
    pub id: WorkerId,
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
            id: wid,
            results: startup_context.results.clone(),
            stats: startup_context.stats.clone(),
        };
        Ok(state)
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
            RunnerMsg::Run(work, sender) => {
                let stat = state.stats.clone();
                let results = state.results.clone();

                let mut job = work.job.clone();

                job.register(stat.clone());

                info!(
                    "Job({})::run({}) with {} tasks",
                    wid,
                    job.name,
                    job.middle.len() + 2
                );

                let job_tag = format!("job#{}", job.id);
                let _ = cast!(stat, StatsMsg::New(job_tag.clone()))?;

                let mut data = vec![];

                // insert tasks into the pipeline
                //
                let p = job.producer.clone();
                let c = job.consumer.clone();
                let first = vec![Task::from(p)];
                let filters = job
                    .middle
                    .iter()
                    .map(|t| Task::from(t.clone()))
                    .collect::<Vec<Task>>();
                let last = vec![Task::from(c)];

                // Create our list of linked tasks
                //
                let mut task_list = [first, filters, last]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<Task>>();
                debug!("task_list: {:?}", task_list);

                // Set the pipeline up
                //
                let (key, stdout) = std::sync::mpsc::channel::<String>();

                trace!("create pipeline");

                // Gather results for all tasks into a single pipeline using `Iterator::fold()`
                //
                let output = stream::iter(task_list.iter_mut())
                    .fold(stdout, async move |acc, t| {
                        let (rx, _h) = t.run(acc).await;
                        rx
                    })
                    .await;

                trace!("starting pipe");

                // Start the pipeline
                //
                let _ = key.send("start".to_string())?;

                // Close the pipeline which will stop all threads in sequence
                //
                drop(key);

                // Wait for the final output to be received and send it out
                //
                for msg in output {
                    write!(&mut data, "{}", msg)?;
                }

                // Fetch stats for the specific job run
                //
                let stats = call!(stat, |port| StatsMsg::Get(job_tag.clone(), port))?;
                let _ = cast!(results, ResultsMsg::Submit(work.id(), stats.clone()))?;
                let _ = sender.send(stats.clone());

                // Clean stats
                //
                let _ = cast!(stat, StatsMsg::Reset(job_tag))?;

                // Update status.
                //
                let _ = work.tx.send(());
            }
        }
        Ok(key)
    }
}

#[derive(Clone, Debug)]
pub struct RunnerBuilder {
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
                results: self.results.clone(),
                stats: self.stat.clone(),
            },
        )
    }
}
