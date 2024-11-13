//! This module will have the various Actors this crate use.
//!

use chrono::Utc;
use ractor::{pg, Actor, ActorProcessingErr, ActorRef};
use std::fmt::{Display, Formatter};
use std::time::{Duration, Instant};
use tracing::{info, trace};

use crate::Stats;

/// Name of the Actor "process group"
const PG_SOURCES: &str = "fetiche_sources";

// -----
pub struct StatsActor;

#[derive(Debug)]
pub enum StatOps {
    /// stat updates
    Pkts(u32),
    Bytes(u64),
    Reconnect,
    Empty,
    Error,
    /// commands
    Reset,
    Print,
    Exit,
}

#[derive(Debug)]
struct State {
    pub start: i64,
    pub stat: Stats,
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "start={} {}", self.start, self.stat)
    }
}

/// stats gathering actor.  You run one actor per task, each with a different `tag`
///
impl Actor for StatsActor {
    type Msg = StatOps;
    type State = State;
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let name = myself.get_name().unwrap();
        trace!("{name} starting.");

        // Register ourselves
        //
        pg::join(PG_SOURCES.into(), vec![myself.get_cell()]);
        Ok(State {
            start: Utc::now().timestamp(),
            stat: Stats::default(),
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            // updates
            StatOps::Pkts(n) => state.stat.pkts += n,
            StatOps::Empty => state.stat.empty += 1,
            StatOps::Error => state.stat.err += 1,
            StatOps::Reconnect => state.stat.reconnect += 1,
            StatOps::Bytes(n) => state.stat.bytes += n,
            // commands
            StatOps::Print => {
                state.stat.tm = (Utc::now().timestamp() - state.start) as u64;
                info!("Stats: {}", state);
            }
            StatOps::Reset => {
                state.stat = Stats::default();
            }
            // The end
            StatOps::Exit => {
                state.stat.tm = (Utc::now().timestamp() - state.start) as u64;
                myself.kill();
            }
        }
        Ok(())
    }
}
