//! Engine handle and operations for a single running instance.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use object_store::local::LocalFileSystem;
use ractor::ActorRef;
use ractor::factory::FactoryMessage;
use strum::EnumString;

use crate::actors::{
    ResultsMsg, RunnerMsg, SchedulerMsg, SourcesMsg, StateMsg, StatsMsg, SuperMsg, TokenMsg,
};
use crate::{Storage, Workspace};

use self::lifecycle::Lifecycle;

pub use parse::Freq;

mod jobs;
mod lifecycle;
mod parse;
mod startup;
mod subr;

/// The Engine struct serves as the core runtime component responsible for managing and coordinating
/// various services and actors within the system. It handles the initialization, coordination, and
/// lifecycle management of multiple concurrent processes and data flows.
///
/// # Architecture
///
/// The Engine operates through a combination of actors and services:
/// - A supervisor actor that oversees all other actors
/// - A factory for managing job runners
/// - A job queue for task scheduling
/// - Source management for data ingestion
/// - State management for persistence
/// - Statistics gathering for monitoring
///
/// # Components
///
/// The Engine maintains several critical components:
/// - Process identification and management
/// - Storage systems for both temporary and persistent data
/// - Token management for authentication
/// - Actor-based concurrent processing system
/// - State synchronization mechanisms
///
#[derive(Clone, Debug)]
pub struct Engine {
    /// Running mode
    mode: EngineMode,
    read_max_bytes: NonZeroU32,
    lifecycle: Arc<Lifecycle>,
    /// Current process DI
    pub pid: u32,
    /// Main area where state is saved (PID, jobs, etc.)
    pub home: Arc<LocalFileSystem>,
    /// Workspace for jobs and other data
    ws: Workspace,
    /// Working area where data is fetched into, etc.
    pub workdir: PathBuf,
    /// Storage area for long-running jobs
    pub storage: Arc<Storage>,
    /// Actor that owns the engine's persistent token storage.
    pub tokens: ActorRef<TokenMsg>,
    // -- actors
    /// Supervisor actor, top of the process group
    pub supervisor: ActorRef<SuperMsg>,
    /// This is the actual scheduler
    pub scheduler: ActorRef<SchedulerMsg>,
    /// Factory for running the actual jobs
    pub factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,
    /// Job results actor
    pub results: ActorRef<ResultsMsg>,
    /// Sources
    pub sources: ActorRef<SourcesMsg>,
    /// Current state
    pub state: ActorRef<StateMsg>,
    /// Stats gathering actors for sources
    pub stats: ActorRef<StatsMsg>,
}

/// Engine can be instantiated into two modes:
/// - `Single` means we will run one job and exit
/// - `Daemon` configures a long-running engine with a worker pool.
///
#[derive(Clone, Copy, Default, Debug, EnumString, strum::Display, PartialEq)]
pub enum EngineMode {
    #[default]
    Single,
    Daemon,
}
