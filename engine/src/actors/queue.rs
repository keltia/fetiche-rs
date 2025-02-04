//! Actor managing the job queue
//!

use crate::{EngineStatus, Job, JobState, ENGINE_PG};
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use std::collections::VecDeque;

/// Messages handled by the QueueActor for managing the job queue.
///
#[derive(Debug)]
pub enum QueueMsg {
    /// Adds a new job to the queue.
    Add(Job),
    /// Gets the next available job ID. Returns the ID through the reply port.
    Allocate(RpcReplyPort<usize>),
    /// Retrieves a job by its ID. Returns the job through the reply port if found.
    GetById(usize, RpcReplyPort<Job>),
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
    /// The queue itself.
    q: VecDeque<Job>,
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

    /// Prepares the `QueueActor` to start.
    ///
    /// This method is called before the actor starts processing messages,
    /// allowing for setup or initialization operations. In this implementation,
    /// the actor joins the `ENGINE_PG` process group and initializes its state
    /// with a new `JobQueue`.
    ///
    /// # Parameters
    /// - `myself`: A reference to the actor itself. This can be used to perform actions or interactions involving the actor.
    /// - `_args`: Arguments provided at initialization. For this actor, an empty tuple `()` is expected.
    ///
    /// # Returns
    /// a `JobQueue` that will represent the actor's initial state.
    ///
    /// # Errors
    /// May return an `ActorProcessingErr` if the initialization process fails.
    ///
    #[tracing::instrument(skip(self))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);

        Ok(QueueState {
            last: args,
            q: VecDeque::new(),
        })
    }

    /// Handles the incoming messages sent to the `QueueActor`.
    ///
    /// This method processes the different variants of the `QueueMsg` enum.
    /// Each variant corresponds to a specific operation on the actor's
    /// internal state (`QueueState`).
    ///
    /// # Parameters
    /// - `myself`: A reference to the actor itself.
    /// - `message`: The `QueueMsg` received by the actor.
    /// - `state`: A mutable reference to the current state of the actor.
    ///
    /// # Returns
    /// A `Result` that indicates whether the message was processed successfully.
    ///
    /// # Possible Message Handling
    ///
    /// - `QueueMsg::Add`: Adds a new job to the queue.
    /// - `QueueMsg::Allocate`: Responds with the next job ID (`state.last`).
    /// - `QueueMsg::GetById`: Returns the corresponding job with its ID.
    /// - `QueueMsg::RemoveId`: Removes a job by its ID.
    /// - `QueueMsg::List`: Responds with the list of all job IDs stored in the queue.
    /// - `QueueMsg::Run`: Remove from the queue and return the job for execution.
    ///
    /// # Panics
    /// If the message variant is not implemented in the match statement, the
    /// method will panic with a runtime error.
    ///
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
            QueueMsg::Add(job) => {
                let mut queued = job.clone();
                if job.state != JobState::Ready {
                    return Err(EngineStatus::JobNotReady(job.id).into());
                }

                queued.state = JobState::Queued;
                state.q.push_back(job);
            }
            QueueMsg::GetById(id, sender) => {
                let job = match state.q.get(id) {
                    Some(job) => job,
                    None => return Err(ActorProcessingErr::from("Job not found {id}")),
                };
                sender.send(job.clone())?;
            }
            QueueMsg::List(sender) => {
                let list = state.q.iter().map(|j| j.id).collect::<Vec<usize>>();
                sender.send(list)?;
            }
            QueueMsg::Run(sender) => {
                let mut job = state.q.pop_front().unwrap();
                if job.state != JobState::Queued {
                    return Err(EngineStatus::JobInWrongState(job.id).into());
                }

                job.state = JobState::Running;
                sender.send(job)?;
                job.state = JobState::Completed;
            }
            QueueMsg::RemoveById(id) => {
                state.q.remove(id);
            }
            _ => panic!(),
        }
        Ok(())
    }
}
