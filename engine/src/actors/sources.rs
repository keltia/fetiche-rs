//! Actor maintaining the different sources we have loaded.
//!

use crate::ENGINE_PG;
use fetiche_sources::Sources;
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

/// Messages handled by the `SourcesActor`.
///
/// The `SourcesMsg` enum defines various types of requests that can be sent
/// to the `SourcesActor` to manage and interact with the loaded `Sources`.
///
/// Variants:
/// - `Get(String)`: Retrieves a specific source by its identifier.
/// - `Count(RpcReplyPort<usize>)`: Returns the total number of sources.
/// - `List(RpcReplyPort<Sources>)`: Returns a complete list of all loaded sources.
/// - `Table(RpcReplyPort<String>)`: Generates a table representation of the sources
///   and sends it as a string.
///
/// Each variant corresponds to a specific behavior implemented within the actor.
///
/// Example usage:
/// ```rust
/// use ractor::RpcReplyPort;
/// use fetiche_engine::SourcesMsg;
///
/// let count_msg = SourcesMsg::Count(RpcReplyPort::default());
/// ```
///
#[derive(Debug)]
pub enum SourcesMsg {
    Get(String),
    Count(RpcReplyPort<usize>),
    List(RpcReplyPort<Sources>),
    Table(RpcReplyPort<String>),
}

/// The actor responsible for managing and interacting with loaded sources.
///
/// The `SourcesActor` provides functionality to interface with the `Sources`
/// data structure, including retrieving specific data, counting existing sources,
/// listing all sources, and generating a table representation of the sources.
///
/// Messages handled by the `SourcesActor`:
/// - `SourcesMsg::Get(String)`: Retrieves a specific source by its identifier.
/// - `SourcesMsg::Count(RpcReplyPort<usize>)`: Returns the total number of sources.
/// - `SourcesMsg::List(RpcReplyPort<Sources>)`: Returns a full list of loaded sources.
/// - `SourcesMsg::Table(RpcReplyPort<String>)`: Returns the data in a table format.
///
pub struct SourcesActor;


#[ractor::async_trait]
impl Actor for SourcesActor {
    type Msg = SourcesMsg;
    type State = Sources;
    type Arguments = ();

    ///
    /// Pre-start hook for the `SourcesActor`.
    ///
    /// The `pre_start` method is invoked before the main actor loop begins and is used to initialize
    /// the actor's state. This includes setting up any resources, connections, or data structures
    /// that the actor will require during its lifecycle.
    ///
    /// # Parameters
    /// - `myself`: A reference to the current actor's `ActorRef`. This can be used to interact with
    ///   the actor itself, such as for further initialization steps.
    /// - `_args`: The arguments provided when starting the actor. These are not used in this implementation.
    ///
    /// # Returns
    /// Returns a `Result` that contains the initial actor state (`Sources`) if successful,
    /// or an `ActorProcessingErr` if an error occurs during initialization.
    ///
    /// # Behavior
    /// - The `SourcesActor` joins the actor system's process group identified by `ENGINE_PG`.
    /// - Initializes the actor's state as a new instance of the `Sources` data structure.
    ///
    /// # Errors
    /// This function may return an `ActorProcessingErr` if:
    /// - The `Sources::new()` function fails to initialize the state.
    ///
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);

        let sources = Sources::new()?;
        Ok(sources)
    }

    /// Handles incoming messages sent to the `SourcesActor`.
    ///
    /// The `handle` function processes various types of messages defined in the `SourcesMsg`
    /// enum and updates or queries the actor's state (`Sources`) accordingly.
    ///
    /// # Parameters
    /// - `_myself`: Reference to the current actor's `ActorRef`, though it is not used in this implementation.
    /// - `message`: The message of type `SourcesMsg` received by the actor.
    /// - `state`: Mutable reference to the actor's internal state (`Sources`).
    ///
    /// # Supported Messages
    /// - `SourcesMsg::Get(key)`: Retrieves a specific source by its identifier. (Not yet implemented.)
    /// - `SourcesMsg::List(sender)`: Returns a full copy of the `Sources` structure by sending it through the supplied `RpcReplyPort`.
    /// - `SourcesMsg::Table(sender)`: Generates a table representation of the sources in the state and sends it as a string via the supplied `RpcReplyPort`.
    /// - `SourcesMsg::Count(sender)`: Calculates the total number of sources in the actor's current state and sends the count via the supplied `RpcReplyPort`.
    ///
    /// # Errors
    /// If there is an error when attempting to send the response through the `RpcReplyPort`,
    /// or if there is a failure in generating the table representation, an `ActorProcessingErr` may occur.
    ///
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SourcesMsg::Get(key) => {
                todo!()
            }
            SourcesMsg::List(sender) => {
                let sources = state.clone();
                sender.send(sources)?;
            }
            SourcesMsg::Table(sender) => {
                let sources = state.clone();
                let table = sources.list()?;
                sender.send(table)?;
            }
            SourcesMsg::Count(sender) => {
                let sources = state.clone();
                let res = sources.len();
                sender.send(res)?;
            }
        }
        Ok(())
    }
}
