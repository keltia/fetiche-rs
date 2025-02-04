//! Actor definition for `Stats`
//!
//! We have different statistics in parallel now, just use New with a tag.
//!

use chrono::Utc;
use nom::Parser;
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::ops::AddAssign;
use tracing::{info, trace};

use crate::{Stats, StatsError, ENGINE_PG};

pub struct StatsActor;

///
#[derive(Debug)]
pub enum StatsMsg {
    /// New session
    New(String),
    /// stat updates
    Update(String, Stats),
    /// commands
    Get(String, RpcReplyPort<Stats>),
    List(RpcReplyPort<String>),
    Reset(String),
    Print(String),
    Exit(String, RpcReplyPort<Stats>),
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
/// start=1696538415 stats=[job#1=pkts=120 bytes=10240 errors=3 reconnects=2
/// job#3=pkts=42 bytes=49152 errors=1 reconnects=0
/// ]
/// ```
///
#[derive(Debug)]
pub struct State {
    pub start: i64,
    pub stats: BTreeMap<String, Stats>,
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "start={} stats=[{}]",
               self.start,
               self.stats.keys().fold(String::new(), |acc, k| acc + &format!("{}={}\n", k, self.stats.get(k).unwrap())))
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
            stats: BTreeMap::new(),
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
            // New session
            StatsMsg::New(name) => {
                state.stats.insert(name, Stats::default());
            }
            // updates
            //
            StatsMsg::Update(tag, stat) => {
                let s = state.stats.get(&tag).unwrap_or(&Stats::default()).clone();
                let new = s + stat;
                let _ = state.stats.insert(tag.clone(), new.clone());
            }
            // commands
            //
            StatsMsg::Get(tag, sender) => {
                let mut s = state.stats.get(&tag).unwrap_or(&Stats::default()).clone();
                s.tm = (Utc::now().timestamp() - state.start) as u64;
                sender.send(s.clone())?;
            }
            StatsMsg::List(sender) => {
                let list = state.stats.keys().fold(String::new(), |acc, k| acc + &format!("{},", k));
                sender.send(list)?;
            }
            StatsMsg::Print(tag) => {
                let mut s = state.stats.get(&tag).unwrap_or(&Stats::default()).clone();
                s.tm = (Utc::now().timestamp() - state.start) as u64;
                info!("Stats: {}", s);
            }
            StatsMsg::Reset(tag) => {
                let _ = state.stats.insert(tag.clone(), Stats::default());
            }
            // The end
            StatsMsg::Exit(tag, sender) => {
                let mut s = state.stats.get(&tag).unwrap_or(&Stats::default()).clone();
                s.tm = (Utc::now().timestamp() - state.start) as u64;
                sender.send(s.clone())?;
                myself.kill();
            }
        }
        Ok(())
    }
}
