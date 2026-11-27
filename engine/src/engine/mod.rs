//! Engine handle and operations for a single running instance.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;

use eyre::Result;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use ractor::ActorRef;
use ractor::factory::FactoryMessage;
use strum::EnumString;

use crate::actors::{
    ResultsMsg, RunnerMsg, STATE_FILE, SchedulerMsg, SourcesMsg, StateMsg, StatsMsg, SuperMsg,
    TokenMsg,
};
use crate::config::ENGINE_CONFIG;
use crate::{SOURCES_CONFIG, Storage, Workspace, version};

use self::lifecycle::Lifecycle;

pub use parse::Freq;

mod catalog;
mod jobs;
mod lifecycle;
mod parse;
mod startup;
mod tokens;

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

impl Engine {
    /// Returns the path of the default state file in the engine's base directory
    ///
    /// This method combines the engine's home directory with the STATE_FILE constant
    /// to provide the full path where state information should be persisted.
    ///
    #[inline]
    pub fn state_file(&self) -> Result<PathBuf> {
        Ok(self.home.path_to_filesystem(&Path::from(STATE_FILE))?)
    }

    /// Returns the path to the sources configuration in the engine home directory.
    #[inline]
    pub fn sources_file(&self) -> Result<PathBuf> {
        Ok(self.home.path_to_filesystem(&Path::from(SOURCES_CONFIG))?)
    }

    /// Returns the full path to the engine's configuration file
    ///
    /// Combines the engine's home directory with the ENGINE_CONFIG constant
    /// to locate the HCL configuration file.
    ///
    #[tracing::instrument(skip(self))]
    pub fn config_file(&self) -> Result<PathBuf> {
        Ok(self.home.path_to_filesystem(&Path::from(ENGINE_CONFIG))?)
    }

    /// Returns a clone of the engine's workspace configuration
    ///
    /// This method provides access to the engine's workspace settings by returning
    /// a copy of the WorkSpace instance, which contains information about the
    /// working environment.
    ///
    #[tracing::instrument(skip(self))]
    pub fn ws(&self) -> Workspace {
        self.ws.clone()
    }

    /// Returns a string containing version information for the engine and its modules
    ///
    /// Formats a string that includes the engine's version number along with
    /// version information for the formats and common modules.
    ///
    pub fn version(&self) -> String {
        format!(
            "{} ({} {})",
            version(),
            fetiche_formats::version(),
            fetiche_common::version(),
        )
    }
}
