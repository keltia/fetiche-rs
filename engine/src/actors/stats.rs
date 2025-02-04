//! Actor definition for `Stats`
//!

use std::fmt::{Display, Formatter};

use chrono::Utc;
use ractor::{pg, Actor, ActorProcessingErr, ActorRef};
use tracing::{info, trace};

use crate::{Stats, ENGINE_PG};

pub struct StatsActor;

/// Represents different types of messages handled by the `StatsActor`.
///
/// These messages allow the actor to update statistics, perform commands, or handle exit sequences.
///
/// # Variants
///
/// * `Pkts(u32)`
///     - Updates the count of packets received. The argument specifies the number of packets to increment.
/// * `Bytes(u64)`
///     - Updates the count of bytes received. The argument specifies the number of bytes to increment.
/// * `Reconnect`
///     - Increments the count of reconnect attempts.
/// * `Error`
///     - Increments the count of errors encountered.
/// * `Reset`
///     - Resets all accumulated statistics back to their default state.
/// * `Print`
///     - Logs the current statistics in a human-readable format. Includes the total runtime and detailed stats.
/// * `Exit`
///     - Signals the actor to terminate. Final statistics are logged before termination.
///
/// >NOTE: as the main users of this are inside `sources`, we need to have these defined here, not
/// the above module that is the `Engine`.
///
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

/// State is a structure representing the current state of the `StatsActor` actor.
///
/// This structure is used to store the start timestamp of the `StatsActor` and its
/// accumulated statistics (`Stats` struct).
///
/// # Fields
///
/// - `start`: A timestamp indicating when the actor started. Stored as an `i64` (Unix timestamp).
/// - `stat`: The `Stats` structure holding counters for packets, bytes, reconnect attempts, errors, etc.
///
/// # Display Implementation
///
/// Implements the `Display` trait to output the actor's state in a human-readable format,
/// combining the start time and current statistics.
///
/// Example output:
/// ```text
/// start=1696538415 pkts=120 bytes=10240 errors=3 reconnects=2
/// ```
///
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
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);
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
