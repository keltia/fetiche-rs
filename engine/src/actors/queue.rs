//! Actor managing the job queue
//!

use crate::{Job, JobQueue, ENGINE_PG};
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

/// Messages handled by the `QueueActor`.
///
/// The `QueueMsg` enum defines various operations that can be performed on the job queue.
///
/// Variants:
/// - `Add`: Add a new job to the queue.
/// - `List`: Retrieve and display the current jobs in the queue.
/// - `Remove`: Remove a job from the queue.
///
#[derive(Debug)]
pub enum QueueMsg {
    Add(Job),
    List(RpcReplyPort<Vec<usize>>),
    Remove(Job),
    RemoveId(usize),
}

pub struct QueueActor;

#[ractor::async_trait]
impl Actor for QueueActor {
    type Msg = QueueMsg;
    type State = JobQueue;
    type Arguments = ();

    /// The actor implementation for `QueueActor` which manages a job queue.
    ///
    /// # Associated Types
    ///
    /// * `Self::Msg`: The type of messages the actor can process. In this case, `QueueMsg`.
    /// * `Self::State`: The actor's internal state, which is the `JobQueue`.
    /// * `Self::Arguments`: Arguments provided during initialization. Here, it's an empty tuple `()`.
    ///
    /// # Overridden Methods
    ///
    /// * `pre_start`: This method is called before the actor starts. Joins the actor to the `ENGINE_PG` process group and initializes the actor state with a new `JobQueue`.
    /// * `handle`: Processes incoming messages defined by `QueueMsg`. The implementation will handle adding, listing, and removing jobs from the queue.
    ///
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);
        Ok(JobQueue::new())
    }

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
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            QueueMsg::List(sender) => sender.send(state.list()),
            _ => panic!(),
        }
    }
}
