//! Library implementing common part of the transformations
//!
//! # Config
//!
//! v2: Has basedir and storage components.
//! v2: Has initial number of workers for the runner factory.
//!
//! Example:
//! ```rust
//! # #[tokio::main]
//! # async fn main() -> eyre::Result<()> {
//! use tracing::trace;
//! use fetiche_engine::Engine;
//!
//! // Instantiate Engine (in daemon mode).
//! // For single usage mode, use `single()`.
//! //
//! let engine = Engine::new().await?;
//!
//! println!("Engine initialised and running.");
//! println!("{}", engine.list_tokens().await?);
//! # Ok(())
//! # }
//! ```
//!
//! In the `Engine`, we run jobs.  `Jobs` are made from a list of `Task` and all tasks are put into
//! a pipeline.  All tasks must be `Runnable` and the `RunnableDerive` proc-macro stitches everything
//! together with channels.
//!
//! Most jobs will be fetch or stream with a conversion task at the end, etc.
//! For the first task, the stdin channel will just serve as a trigger for the pipeline.
//!
//! Each `Runnable` task will be marked as `RunnableDerive` and will need to define an `execute()`
//! member function for the main task.  It takes the previous stage output as a string and should
//! return a string with the transformed output that will be sent to the next stage.
//!
use std::collections::BTreeMap;
use std::env;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eyre::Result;
use futures_util::StreamExt;
use object_store::local::LocalFileSystem;
use object_store::ObjectStore;
use ractor::factory::{queues, routing, Factory, FactoryArguments, FactoryMessage};
use ractor::registry::registered;
use ractor::{call, cast, Actor, ActorRef};
use serde::Deserialize;
use strum::EnumString;
use tokio::fs;
use tracing::{debug, error, info, trace};

pub use auth::*;
pub use consumer::*;
pub use error::*;
pub use filter::*;
pub use job::*;
pub use middle::*;
pub use parse::*;
pub use producer::*;
pub use sources::*;
pub use storage::*;
pub use task::*;
pub use tokens::TokenStorage;

use crate::actors::*;

use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;

mod actors;
mod auth;
mod cmds;
mod consumer;
mod error;
mod filter;
mod job;
mod middle;
mod parse;
mod producer;
mod sources;
mod stats;
mod storage;
mod subr;
mod task;
mod tokens;

/// Engine signature
///
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// Configuration filename
const ENGINE_CONFIG: &str = "engine.hcl";

/// Current running process ID — We have a separate forked engine
const ENGINE_PID: &str = "acutectl.pid";

/// Configuration file version
const ENGINE_VERSION: usize = 3;

/// Engine process group for the actors
const ENGINE_PG: &str = "engine.pg";

/// Clock tick
const TICK: Duration = Duration::from_secs(2);

/// Sync state every 30s by default.
const SYNC: Duration = Duration::from_secs(30);

/// The `EngineConfig` struct provides the configuration options used to initialize
/// and manage the `Engine`. It is loaded from the `engine.hcl` file or other sources
/// and defines the base runtime parameters for the engine.
///
/// # Fields
///
/// - `basedir`
///     The base directory where engine-related files, such as state, jobs, and tasks,
///     are stored. This path acts as the root directory for engine operations.
///
/// - `workers`
///     Initial number of runner spawned by the factory.
///
/// - `storage`
///     A `BTreeMap` defining various storage configurations. This can include
///     settings for in-memory caching, directory-based storage, or Hive-based
///     sharding. Each storage configuration must conform to the `StorageConfig` enum.
///
#[into_configfile(version = 3, filename = "engine.hcl")]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct EngineConfig {
    /// Base directory
    pub basedir: PathBuf,
    /// Number of workers for the runner factory.
    pub workers: Option<usize>,
    /// Sync the engine state every 30 seconds by default
    pub sync: Option<Duration>,
    /// This is our clock: we get a message every 2 seconds by default
    pub tick: Option<Duration>,
    /// List of storage types
    pub storage: BTreeMap<String, StorageConfig>,
}

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
    /// Current process DI
    pub pid: u32,
    /// Main area where state is saved (PID, jobs, etc.)
    pub home: Arc<LocalFileSystem>,
    /// Working area where data is fetched into, etc.
    pub workdir: PathBuf,
    /// Storage area for long-running jobs
    pub storage: Arc<Storage>,
    /// Storage areas for auth tokens
    pub tokens: Arc<TokenStorage>,
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
/// - `Daemon` means we will be part of a daemon (`fetiched`).
///
#[derive(Clone, Copy, Default, Debug, EnumString, strum::Display, PartialEq)]
pub enum EngineMode {
    #[default]
    Single,
    Daemon,
}

impl Engine {
    /// Creates a new Engine instance in daemon mode with configuration loaded from engine.hcl
    ///
    /// This method initializes an Engine configured for long-running daemon operation.
    /// It loads configuration from the engine.hcl file and sets up all necessary components
    /// including storage, actors, and state management systems.
    ///
    /// The daemon mode enables features like:
    /// - Multiple concurrent worker threads
    /// - Periodic state synchronization
    /// - Regular system health checks via tick intervals
    ///
    /// # Errors
    ///
    /// Will panic if the Engine cannot be created due to configuration or initialization errors.
    ///
    #[tracing::instrument]
    pub async fn new() -> Result<Self> {
        // Load storage areas from `engine.hcl`
        //
        Self::load(ENGINE_CONFIG, EngineMode::Daemon).await
    }

    /// Creates a new Engine instance in single mode with configuration loaded from engine.hcl
    ///
    /// This method initializes an Engine configured for single-job execution mode.
    /// It loads configuration from the engine.hcl file and sets up the necessary components
    /// with minimal resources since it will only process one job before exiting.
    ///
    /// # Errors
    ///
    /// Will panic if the Engine cannot be created due to configuration or initialization errors.
    ///
    #[tracing::instrument]
    pub async fn single() -> Result<Self> {
        // Load storage areas from `engine.hcl`
        //
        Self::load(ENGINE_CONFIG, EngineMode::Single).await
    }

    /// Creates a new Engine instance by loading configuration from the specified file
    ///
    /// # Arguments
    ///
    /// * `fname` - Path to the configuration file to load
    /// * `mode` - Operating mode for the engine (Single or Daemon)
    ///
    /// # Returns
    ///
    /// Returns a Result containing the initialized Engine instance if successful
    ///
    /// # Errors
    ///
    /// Will return an error if:
    /// - The configuration file cannot be loaded or parsed
    /// - The configuration version doesn't match the expected version
    /// - Any of the engine components (actors, storage, etc.) fail to initialize
    /// - Required directories cannot be created or accessed
    ///
    #[tracing::instrument]
    pub async fn load(fname: &str, mode: EngineMode) -> Result<Self> {
        info!("Engine v{} starting", env!("CARGO_PKG_VERSION"));
        info!("Starting in {} mode", mode);

        let root = ConfigFile::<EngineConfig>::load(Some(fname))?;
        let cfg = root.inner();
        let home = root.config_path();
        info!("Home is in {home:?}");

        // Bail out if different
        //
        if cfg.version() != ENGINE_VERSION {
            error!("Bad config version {}", cfg.version());
            return Err(EngineStatus::BadConfigVersion(cfg.version(), ENGINE_VERSION).into());
        }

        // Ensure we have sensible defaults.
        //
        let (workers, sync, tick) = if mode == EngineMode::Daemon {
            let workers =
                cfg.workers
                    .unwrap_or_else(|| match std::thread::available_parallelism() {
                        Ok(n) => n.get(),
                        Err(_) => 1,
                    });
            let sync = cfg.sync.unwrap_or(SYNC);
            let tick = cfg.tick.unwrap_or(TICK);
            (workers, sync, tick)
        } else {
            // When running as a single instance, we have no need for multiple workers or a 2s tick
            //
            (1, SYNC, Duration::from_secs(1))
        };

        debug!("Engine config: {:#?}", cfg);

        let pid = std::process::id();
        info!("Engine PID={}", pid);

        // Create our object storage "vision" out of our base directory.
        //
        let base = Arc::new(LocalFileSystem::new_with_prefix(home)?);

        // Move ourselves into our base
        //
        // BASEDIR/var/run/<PID> for single instance runs
        // BASEDIR/var/run/acute for fetiched runs
        //
        let workdir = if mode == EngineMode::Single {
            cfg.basedir.join("var").join("run").join(pid.to_string())
        } else {
            cfg.basedir.join("var").join("run").join("acute")
        };
        fs::create_dir_all(&workdir).await?;

        // ----- Start actors

        // We have a generic supervisor actor.
        //
        trace!("starting supervisor actor aka init.");
        let tag = String::from("init");
        let (sup, _h) = Actor::spawn(Some(tag), Supervisor, ()).await?;

        // Start the stats gathering service.
        //
        trace!("starting stats actor.");
        let tag = String::from("engine::stats");
        let (stat, _h) =
            Actor::spawn_linked(Some(tag), StatsActor, "sources".into(), sup.get_cell()).await?;

        // Start sources service
        //
        trace!("load sources");
        let (src, _h) = Actor::spawn_linked(
            Some("engine::sources".into()),
            SourcesActor,
            (),
            sup.get_cell(),
        )
        .await?;

        let count = call!(src, SourcesMsg::Count)?;
        info!("{} sources loaded", count);

        // Start state service
        //
        trace!("load state.");
        let (state, _h) = Actor::spawn_linked(
            Some("engine::state".into()),
            StateActor,
            base.clone(),
            sup.get_cell(),
        )
        .await?;
        trace!("state={:?}", state);

        // Get last used ID from the previous state
        //
        let last = call!(state, StateMsg::Last)?;

        trace!("load results");
        let (results, _h) = Actor::spawn_linked(
            Some("engine::results".into()),
            ResultsActor,
            (),
            sup.get_cell(),
        )
        .await?;

        // ----- Start Runner Factory

        let factory_def = Factory::<
            usize,
            RunnerMsg,
            RunnerArgs,
            RunnerActor,
            routing::QueuerRouting<usize, RunnerMsg>,
            queues::DefaultQueue<usize, RunnerMsg>,
        >::default();
        let runner_builder = RunnerBuilder {
            results: results.clone(),
            stat: stat.clone(),
        };
        let factory_args = FactoryArguments::builder()
            .worker_builder(Box::new(runner_builder))
            .queue(Default::default())
            .router(Default::default())
            .num_initial_workers(workers)
            .build();

        // Spawn factory under supervision too.
        //
        let (factory, _h) = Actor::spawn_linked(
            Some("factory".into()),
            factory_def,
            factory_args,
            sup.get_cell(),
        )
        .await?;

        // Spawn the actual scheduler
        //
        let sargs = SchedulerArguments {
            sync,
            tick,
            last,
            state: state.clone(),
            results: results.clone(),
            factory: factory.clone(),
        };
        let (scheduler, _h) = Actor::spawn_linked(
            Some("scheduler".into()),
            SchedulerActor,
            sargs,
            sup.get_cell(),
        )
        .await?;

        // ----- Register non-actor subsystems

        // Register storage areas
        //
        trace!("load storage areas");
        let areas = Storage::register(&cfg.storage);
        info!("{} areas loaded", areas.len());

        // Register tokens
        //
        let tokens_area = root
            .config_path()
            .join("tokens")
            .to_string_lossy()
            .to_string();
        trace!("load tokens from {tokens_area}");
        let tokens = TokenStorage::register(&tokens_area).await?;
        info!("{} tokens loaded", tokens.len());

        // Instantiate everything
        //
        let engine = Engine {
            mode,
            pid,
            home: base.clone(),
            workdir: workdir.clone(),
            storage: Arc::new(areas),
            tokens: Arc::new(tokens),
            supervisor: sup.clone(),
            scheduler: scheduler.clone(),
            factory: factory.clone(),
            results: results.clone(),
            sources: src.clone(),
            state: state.clone(),
            stats: stat.clone(),
        };

        // Sync immediately, ensuring the state is clean
        //
        engine.sync()?;

        // If debug/trace, list all the actors running at this point.
        //
        let plist = registered().join("\n");
        trace!("Actor list:\n{plist}");

        // Display the current storage objects.
        //
        trace!("Storage objects:");
        let mut flist = base.list(None);
        while let Some(meta) = flist.next().await.transpose()? {
            trace!("{:?} ({} bytes)", meta.location, meta.size);
        }

        // Get the ball rolling.  Next tick, we will check the queue.
        //
        cast!(scheduler, SchedulerMsg::Start)?;

        Ok(engine)
    }
}
