//! Actor maintaining the different sources we have loaded.
//!

use crate::{Site, Sources, ENGINE_PG};
use eyre::eyre;
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
#[derive(Debug)]
pub enum SourcesMsg {
    Get(String, RpcReplyPort<Site>),
    Count(RpcReplyPort<usize>),
    List(RpcReplyPort<Sources>),
    Reload(RpcReplyPort<Sources>),
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
    /// A `Result` that contains the initial actor state (`Sources`) if successful,
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
    #[tracing::instrument(skip(self))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);

        let src = Sources::new()?;
        Ok(src)
    }

    /// Main message handling method for the `SourcesActor`.
    ///
    /// This method processes incoming messages and updates the actor's state accordingly.
    /// It handles various types of messages defined in the `SourcesMsg` enum.
    ///
    /// # Parameters
    /// - `_myself`: A reference to the current actor instance
    /// - `message`: The message to be processed
    /// - `state`: Mutable reference to the current actor state
    ///
    /// # Returns
    /// Returns `Ok(())` if the message was processed successfully, or an `ActorProcessingErr`
    /// if an error occurred during processing.
    ///
    /// # Message Handling
    /// - `Get`: Retrieves a specific source by key
    /// - `List`: Returns a clone of all sources
    /// - `Table`: Generates a table representation of sources
    /// - `Count`: Returns the total number of sources
    /// - `Reload`: Reloads all sources and updates the state
    ///
    #[tracing::instrument(skip(self, _myself, state))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SourcesMsg::Get(key, sender) => {
                let site = match state.get(&key) {
                    Some(site) => site.clone(),
                    None => {
                        let err = format!("Unknown site: {}", key);
                        tracing::error!("{}", err);
                        return Err(ActorProcessingErr::from(eyre!(err)));
                    }
                };
                sender.send(site)?;
            }
            SourcesMsg::List(sender) => {
                let sources = state.clone();
                sender.send(sources)?;
            }
            SourcesMsg::Table(sender) => {
                let table = state.list()?;
                sender.send(table)?;
            }
            SourcesMsg::Count(sender) => {
                let res = state.len();
                sender.send(res)?;
            }
            SourcesMsg::Reload(sender) => {
                let sources = Sources::new()?;
                sender.send(sources.clone())?;
                *state = sources;
            }
        }
        Ok(())
    }
}
