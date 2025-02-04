//! Actor definition for `Supervisor`
//!

use fetiche_sources::SOURCES_PG;
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use tracing::trace;

#[derive(Debug)]
pub enum SuperMsg {
    Dummy,
}

/// This actor will act as a supervisor to child actors.
///
pub struct Supervisor;

/// Supervisor actor implementation.
///
/// The `Supervisor` actor is responsible for managing child actors and supervising
/// their lifecycle events. It handles messages of type `SuperMsg` which are specifically
/// defined for its operations.
///
/// # Responsibilities
/// - Supervises one or more child actors, handling events like actor termination, startup,
///   process group changes, etc.
/// - Responds to `Dummy` messages for demonstration purposes (currently does not perform
///   significant operations).
///
/// # Message Handling
/// 1. **Handling `SuperMsg`:**
///     - Currently supports the `Dummy` message, logging it for demonstration.
/// 2. **Supervisor Events:**
///     - `ActorTerminated`: Logs the termination of child actors.
///     - `ActorFailed`: Logs the failure of child actors along with error details.
///     - `ProcessGroupChanged`: Logs changes in process groups.
///     - `ActorStarted`: Logs the successful startup of child actors.
///
///
/// # Execution Flow
/// - On startup (`pre_start`):
///     - Joins the process group `PG_SOURCES`.
/// - Message handling (`handle`):
///     - Processes messages of type `SuperMsg`. Default behavior involves handling a
///       `Dummy` message.
/// - Supervisor event handling (`handle_supervisor_evt`):
///     - Processes various lifecycle events of supervised actors.
///
/// # Usage
/// This actor is designed to be integrated as part of a larger system, supervising other
/// actors in a hierarchy. It is lightweight and mainly logs events for tracing purposes.
///
#[ractor::async_trait]
impl Actor for Supervisor {
    type Msg = SuperMsg;
    type State = ();
    type Arguments = ();

    /// Nothing to do on startup.
    ///
    #[tracing::instrument(skip(self, myself))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(SOURCES_PG.into(), vec![myself.get_cell()]);
        Ok(())
    }

    /// We are not doing anything by ourselves.
    ///
    #[tracing::instrument(skip(self, _myself))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SuperMsg::Dummy => {
                trace!("Dummy message received.");
            }
        }
        Ok(())
    }

    /// Handles supervisor events such as actor termination, startup, or process group changes.
    ///
    /// # Parameters
    /// - `_myself`: A reference to the actor itself.
    /// - `message`: The supervision event to handle.
    /// - `_state`: The mutable state associated with the supervisor actor.
    ///
    /// # Supervision Events
    /// This function handles the following lifecycle events:
    /// - **ActorTerminated**: Logs the successful termination of a child actor.
    /// - **ActorFailed**: Logs the failure of a child actor along with the error details.
    /// - **ProcessGroupChanged**: Logs any changes in the actor's process group membership.
    /// - **ActorStarted**: Logs the successful startup of a child actor.
    ///
    /// # Execution Flow
    /// Based on the received `SupervisionEvent`, this method will log the corresponding event
    /// to help trace the lifecycle of supervised actors. It does not perform any additional
    /// operations currently, aside from logging.
    ///
    /// # Errors
    /// This function does not return any errors and always resolves to `Ok(())`.
    ///
    #[tracing::instrument(skip(self, _myself))]
    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorTerminated(cell, ..) => {
                trace!("Actor {} is finished.", cell.get_name().unwrap());
            }
            SupervisionEvent::ActorFailed(cell, err) => {
                trace!("Actor {} terminated with: {err}", cell.get_name().unwrap());
            }
            SupervisionEvent::ProcessGroupChanged(msg) => {
                trace!("Process group changed {msg:?}");
            }
            SupervisionEvent::ActorStarted(cell) => {
                trace!("Actor {} is started.", cell.get_name().unwrap());
            }
        }
        Ok(())
    }
}
