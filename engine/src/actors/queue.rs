//! Actor managing the job queue
//!
use std::collections::VecDeque;

use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use strum::EnumString;
use tracing::trace;

use crate::{Job, JobState, QueueError, ENGINE_PG};

/// Messages handled by the QueueActor for managing the job queue.
///
#[derive(Debug)]
pub enum QueueMsg {
    /// Adds a new job to the waiting queue.
    Add(Job, RpcReplyPort<usize>),
    /// Gets the next available job ID. Returns the ID through the reply port.
    Allocate(RpcReplyPort<usize>),
    /// Check if there is anything to do in any of the queues.
    Empty(RpcReplyPort<bool>),
    /// Move from running into the finished queue.
    Finished(Job),
    /// Lists all job IDs currently in the queue. Returns vector of IDs through the reply port.
    List(RpcReplyPort<Vec<usize>>),
    /// Removes a job from the queue using its ID.
    RemoveById(usize),
    /// Gets and removes the next job from the queue for execution. Returns the job through the reply port.
    Run(RpcReplyPort<Job>),
}

pub struct QueueActor;

#[derive(Debug)]
pub struct QueueState {
    /// Last job ID.
    last: usize,
    /// The queues:
    waiting: VecDeque<Job>,
    running: VecDeque<Job>,
    finished: VecDeque<Job>,
}

/// We have three different queues.
///
#[derive(Debug, EnumString, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum Queue {
    /// Initial queue when submitting a job.
    Waiting,
    /// Next job to run is put here.
    Running,
    /// Then it is moved after completion.
    Finished,
}

/// The actor implementation for `QueueActor` which manages a job queue.
///
/// # Associated Types
///
/// * `Self::Msg`: The type of messages the actor can process. In this case, `QueueMsg`.
/// * `Self::State`: The actor's internal state, which is the `QueueState`.
/// * `Self::Arguments`: Arguments provided during initialization. Here, it is the last ID that was used.
///
/// # Overridden Methods
///
/// * `pre_start`: This method is called before the actor starts. Joins the actor to the `ENGINE_PG` process group and initializes the actor state with a new `JobQueue`.
/// * `handle`: Processes incoming messages defined by `QueueMsg`. The implementation will handle adding, listing, and removing jobs from the queue.
///
#[ractor::async_trait]
impl Actor for QueueActor {
    type Msg = QueueMsg;
    type State = QueueState;
    type Arguments = usize;

    #[tracing::instrument(skip(self, myself))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);

        Ok(QueueState {
            last: args,
            waiting: VecDeque::new(),
            running: VecDeque::new(),
            finished: VecDeque::new(),
        })
    }

    #[tracing::instrument(skip(self, _myself))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            QueueMsg::Allocate(sender) => {
                sender.send(state.last)?;
                state.last += 1;
            }
            QueueMsg::Add(job, port) => {
                let mut queued = job.clone();
                if job.state() != JobState::Ready {
                    return Err(QueueError::JobNotReady(job.id).into());
                }
                trace!("Adding job to waiting queue: {:?}", queued);
                state.waiting.push_back(queued);
                let _ = port.send(job.id)?;
            }
            QueueMsg::List(sender) => {
                let list = state.running.iter().map(|j| j.id).collect::<Vec<usize>>();
                sender.send(list)?;
            }
            QueueMsg::Run(sender) => {
                let mut job = match state.waiting.pop_front() {
                    Some(job) => job,
                    None => return Ok(()),
                };
                trace!("Running next job from queue: {:?}", job);
                state.running.push_back(job.clone());
                sender.send(job)?;
            }
            QueueMsg::Finished(job) => {
                let job = match state.running.pop_front() {
                    Some(job) => job,
                    None => return Ok(()),
                };
                state.finished.push_back(job);
            }
            QueueMsg::RemoveById(id) => {
                state.running.remove(id);
            }
            QueueMsg::Empty(sender) => {
                sender.send(state.waiting.is_empty() && state.running.is_empty() && state.finished.is_empty())?;
            }
        }
        Ok(())
    }
}
