//! This module will have the various Actors this crate use.  Various system-dependant actors will
//! access these.
//!
//! Actors:
//!
//! `Stats`
//!
//! This actor accumulates statistics about packets/bytes received, etc.
//!
//! `Supervisor`
//!
//! This actor will be the father of all actors spawned by `sources`.
//!

use chrono::Utc;
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, SupervisionEvent};
use std::fmt::{Display, Formatter};
use tracing::{info, trace};

use crate::Stats;

/// Name of the Actor "process group"
pub const PG_SOURCES: &str = "fetiche_sources";

// -----

pub struct StatsActor;

#[derive(Debug)]
pub enum StatsMsg {
    /// stat updates
    Pkts(u32),
    Bytes(u64),
    Reconnect,
    Error,
    /// commands
    Reset,
    Print,
    Exit,
}

#[derive(Debug)]
pub struct State {
    pub start: i64,
    pub stat: Stats,
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "start={} {}", self.start, self.stat)
    }
}

/// StatsActor is responsible for managing and tracking statistics related to packet and byte counts,
/// reconnect attempts, errors, and more. This actor processes messages to update the stats, execute
/// commands such as resetting or printing stats, and handle termination requests.
///
/// StatsMsg is the message type used with this actor. Messages may include stat updates
/// (`Pkts`, `Bytes`, `Reconnect`, `Error`) or commands (`Reset`, `Print`, `Exit`).
///
/// The actor maintains a `State` structure that contains the start timestamp and accumulated statistics.
/// This actor is intended to be lightweight and focused on processing statistics for a given task
/// or context.
///
#[ractor::async_trait]
impl Actor for StatsActor {
    type Msg = StatsMsg;
    type State = State;
    type Arguments = String;

    #[tracing::instrument(skip(self, args))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let name = myself.get_name().unwrap();
        trace!("{name}({args}) starting.");

        // Register ourselves
        //
        pg::join(PG_SOURCES.into(), vec![myself.get_cell()]);
        Ok(State {
            start: Utc::now().timestamp(),
            stat: Stats::default(),
        })
    }

    #[tracing::instrument(skip(self))]
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            // updates
            StatsMsg::Pkts(n) => state.stat.pkts += n,
            StatsMsg::Error => state.stat.err += 1,
            StatsMsg::Reconnect => state.stat.reconnect += 1,
            StatsMsg::Bytes(n) => state.stat.bytes += n,
            // commands
            StatsMsg::Print => {
                state.stat.tm = (Utc::now().timestamp() - state.start) as u64;
                info!("Stats: {}", state);
            }
            StatsMsg::Reset => {
                state.stat = Stats::default();
            }
            // The end
            StatsMsg::Exit => {
                state.stat.tm = (Utc::now().timestamp() - state.start) as u64;
                myself.kill();
            }
        }
        Ok(())
    }
}

// -----

/// This actor will act as a supervisor to child actors.
///
pub struct Supervisor;

/// Supervisor is responsible for overseeing and managing the lifecycle
/// of child actors. It monitors their starting, termination, failures,
/// and process group changes, ensuring that any necessary logging or tracking
/// occurs.
///
/// # Responsibilities
///
/// - Adds itself to the process group `PG_SOURCES` upon startup.
/// - Handles events related to child actor lifecycle, such as:
///   - `ActorTerminated`: Child actors' termination is logged.
///   - `ActorFailed`: Logs child actors that terminated with an error.
///   - `ProcessGroupChanged`: Logs changes to the associated process group.
///   - `ActorStarted`: Logs when a child actor starts.
///
/// The `Supervisor` itself does not perform specific tasks or act on direct messages but
/// focuses on its children and their behavior in a process group context.
///
#[ractor::async_trait]
impl Actor for Supervisor {
    type Msg = ();
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
        pg::join(PG_SOURCES.into(), vec![myself.get_cell()]);
        Ok(())
    }

    /// We are not doing anything by ourselves.
    ///
    #[tracing::instrument(skip(self, _myself))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        _message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        Ok(())
    }

    /// All the work is done here.
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
