//! Library implementing common part of the transformations
//!
//! # Config
//!
//! v2: Has basedir and storage components.
//! v2: Has initial number of workers for the runner factory.
//!
//! Example:
//! ```rust,no_run
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
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use eyre::Result;
use futures_util::StreamExt;
use object_store::local::LocalFileSystem;
use object_store::{ObjectStore, ObjectStoreExt};
use ractor::factory::{Factory, FactoryArguments, FactoryMessage, queues, routing};
use ractor::registry::registered;
use ractor::{Actor, ActorRef, call, cast};
use serde::Deserialize;
use strum::EnumString;
use tokio::fs;
use tracing::{error, info, trace};

pub use actors::{TokenMsg, TokenStorage};
pub use auth::*;
pub use consumer::*;
pub use error::*;
pub use job::*;
pub use middle::*;
pub use parse::*;
pub use producer::*;
pub use sources::*;
pub use storage::*;
pub use task::*;
pub use workspace::*;

use crate::actors::*;
use crate::lifecycle::{Lifecycle, Resources};

use fetiche_common::{ConfigFile, Filter, IntoConfig, Versioned, time};
use fetiche_macros::into_configfile;

mod actors;
mod auth;
mod cmds;
mod consumer;
mod error;
mod job;
mod lifecycle;
mod middle;
mod parse;
mod producer;
mod sources;
mod stats;
mod storage;
mod subr;
mod task;
pub mod token;
mod workspace;

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

/// File created into our workdir to indicate something is running.
const CANARY_FILE: &str = "running";

/// Clock tick
const TICK: Duration = Duration::from_secs(2);

/// Sync state every 30s by default.
const SYNC: Duration = Duration::from_secs(30);

const DEFAULT_PIPELINE_QUEUE_CAPACITY: usize = 16;
static NEXT_ENGINE_INSTANCE: AtomicUsize = AtomicUsize::new(0);

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
    /// Number of queued messages per pipeline edge.
    pub pipeline_queue_capacity: Option<usize>,
    /// Maximum file size in bytes for a configured Read stage.
    pub read_max_bytes: Option<NonZeroU32>,
    /// Sync the engine state every 30 seconds by default
    pub sync: Option<Duration>,
    /// This is our clock: we get a message every 2 seconds by default
    pub tick: Option<Duration>,
    /// List of storage types
    pub storage: BTreeMap<String, StorageConfig>,
}

impl EngineConfig {
    /// Validates explicitly configured worker and timer settings, then resolves
    /// the effective settings for the selected mode.
    fn runtime_settings(&self, mode: EngineMode) -> Result<(usize, Duration, Duration)> {
        if self.workers == Some(0) {
            return Err(EngineStatus::ZeroWorkers.into());
        }
        for (name, interval) in [("tick", self.tick), ("sync", self.sync)] {
            if interval == Some(Duration::ZERO) {
                return Err(EngineStatus::ZeroInterval(name).into());
            }
        }
        if mode == EngineMode::Single {
            return Ok((1, SYNC, Duration::from_secs(1)));
        }

        let workers = self.workers.unwrap_or_else(|| {
            std::thread::available_parallelism().map_or(1, |workers| workers.get())
        });
        Ok((
            workers,
            self.sync.unwrap_or(SYNC),
            self.tick.unwrap_or(TICK),
        ))
    }
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
    /// Returns an error if configuration or startup fails.
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
    /// Returns an error if configuration or startup fails.
    ///
    #[tracing::instrument]
    pub async fn single() -> Result<Self> {
        // Load storage areas from `engine.hcl`
        //
        Self::load(ENGINE_CONFIG, EngineMode::Single).await
    }

    /// Starts an engine from the specified configuration file.
    ///
    /// A bare filename is looked up in the default configuration directory.
    /// An explicit path uses its containing directory for sources, tokens, and
    /// state. On startup failure, the engine stops any actors it started and
    /// releases its running marker.
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
    /// - The configured worker count or a scheduler interval is zero
    /// - Any of the engine components (actors, storage, etc.) fail to initialize
    /// - Required directories cannot be created or accessed
    ///
    #[tracing::instrument]
    pub async fn load(fname: &str, mode: EngineMode) -> Result<Self> {
        info!("Engine v{} starting", env!("CARGO_PKG_VERSION"));
        info!("Starting in {} mode", mode);

        let root = ConfigFile::<EngineConfig>::load(Some(fname))?;
        let cfg = root.inner();
        let filename = std::path::Path::new(fname);
        let home = if filename.file_name() == Some(filename.as_os_str()) {
            root.config_path()
        } else {
            fs::canonicalize(filename)
                .await?
                .parent()
                .ok_or_else(|| eyre::eyre!("engine configuration must have a parent directory"))?
                .to_owned()
        };
        info!("Home is in {home:?}");

        // Bail out if different
        //
        if cfg.version() != ENGINE_VERSION {
            error!("Bad config version {}", cfg.version());
            return Err(EngineStatus::BadConfigVersion(cfg.version(), ENGINE_VERSION).into());
        }

        let queue_capacity = cfg
            .pipeline_queue_capacity
            .unwrap_or(DEFAULT_PIPELINE_QUEUE_CAPACITY);
        if !(1..=tokio::sync::Semaphore::MAX_PERMITS).contains(&queue_capacity) {
            return Err(eyre::eyre!(
                "pipeline_queue_capacity must be between 1 and {}",
                tokio::sync::Semaphore::MAX_PERMITS
            ));
        }

        let (workers, sync, tick) = cfg.runtime_settings(mode)?;
        let read_max_bytes = cfg
            .read_max_bytes
            .unwrap_or(producer::DEFAULT_READ_MAX_BYTES);
        info!(
            mode = %mode, workers, sync = ?sync, tick = ?tick, queue_capacity,
            read_max_bytes = read_max_bytes.get(),
            "Resolved engine runtime settings"
        );

        let pid = std::process::id();
        info!("Engine PID={}", pid);

        // Create our object storage "vision" out of our base directory.
        //
        let base = Arc::new(LocalFileSystem::new_with_prefix(&home)?.with_automatic_cleanup(true));

        // Move ourselves into our base
        //
        // BASEDIR/var/run/<PID> for single instance runs
        // BASEDIR/var/run/acute for fetiched runs
        //
        let work_prefix = std::path::absolute(cfg.basedir.join("var").join("run"))?;
        fs::create_dir_all(&work_prefix).await?;
        let ws = Workspace::new(&work_prefix)?;

        let workdir = if mode == EngineMode::Single {
            ws.path().join(pid.to_string())
        } else {
            ws.path().join("acute")
        };
        fs::create_dir_all(&workdir).await?;

        let mut resources = Resources::claim(&workdir)?;
        let instance = NEXT_ENGINE_INSTANCE.fetch_add(1, Ordering::Relaxed);
        let name = |role| Some(format!("engine::{instance}::{role}"));
        let startup = async {
            // ----- Start actors

            // We have a generic supervisor actor.
            //
            trace!("starting supervisor actor aka init.");
            let (sup, handle) = Actor::spawn(name("supervisor"), Supervisor, ()).await?;
            resources.track(&sup, handle);

            // Start the stats gathering service.
            //
            trace!("starting stats actor.");
            let (stat, handle) =
                Actor::spawn_linked(name("stats"), StatsActor, "sources".into(), sup.get_cell())
                    .await?;
            resources.track(&stat, handle);

            // Start sources service
            //
            trace!("load sources");
            let (src, handle) =
                Actor::spawn_linked(name("sources"), SourcesActor, home.clone(), sup.get_cell())
                    .await?;
            resources.track(&src, handle);

            let count = call!(src, SourcesMsg::Count)?;
            info!("{} sources loaded", count);

            // Start state service
            //
            trace!("load state.");
            let (state, handle) =
                Actor::spawn_linked(name("state"), StateActor, base.clone(), sup.get_cell())
                    .await?;
            resources.track(&state, handle);
            trace!("state={:?}", state);

            // Get last used ID from the previous state
            //
            let last = call!(state, StateMsg::Last)?;

            trace!("load results");
            let (results, handle) =
                Actor::spawn_linked(name("results"), ResultsActor, (), sup.get_cell()).await?;
            resources.track(&results, handle);

            // Register tokens
            //
            let tokens_area = home.join("tokens");

            trace!("load tokens from {tokens_area:?}");
            let (tokens, handle) =
                Actor::spawn_linked(name("tokens"), TokenActor, tokens_area, sup.get_cell())
                    .await?;
            resources.track(&tokens, handle);

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
                queue_capacity,
                cancel: resources.cancellation(),
                tokens: Some(tokens.clone()),
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
            let (factory, handle) =
                Actor::spawn_linked(name("factory"), factory_def, factory_args, sup.get_cell())
                    .await?;
            resources.track(&factory, handle);

            // Spawn the actual scheduler
            //
            let sargs = SchedulerArguments {
                sync,
                tick,
                last,
                workers,
                state: state.clone(),
                factory: factory.clone(),
            };
            let (scheduler, handle) =
                Actor::spawn_linked(name("scheduler"), SchedulerActor, sargs, sup.get_cell())
                    .await?;
            resources.track(&scheduler, handle);

            // ----- Register non-actor subsystems

            // Register storage areas
            //
            trace!("load storage areas");
            let areas = Storage::register(&cfg.storage)?;
            info!("{} areas loaded", areas.len());

            call!(state, StateMsg::Flush)??;

            // If debug/trace, list all the actors running at this point.
            //
            let plist = registered().join("\n");
            trace!("Actor list:\n{plist}");

            // Display the current storage objects.
            //
            let mut flist = base.list(None);
            let mut object_count = 0usize;
            while flist.next().await.transpose()?.is_some() {
                object_count += 1;
            }
            trace!(object_count, "Listed storage objects");

            // Get the ball rolling.  Next tick, we will check the queue.
            //
            cast!(scheduler, SchedulerMsg::Start)?;

            base.put(
                &object_store::path::Path::from(ENGINE_PID),
                pid.to_string().into(),
            )
            .await?;
            Ok::<_, eyre::Report>((
                sup, scheduler, factory, results, src, state, stat, areas, tokens,
            ))
        }
        .await;
        let (sup, scheduler, factory, results, src, state, stat, areas, tokens) = match startup {
            Ok(started) => started,
            Err(err) => {
                if let Err(cleanup_error) = resources.teardown().await {
                    tracing::warn!(%cleanup_error, "Engine startup rollback did not finish");
                }
                return Err(err);
            }
        };

        Ok(Self {
            mode,
            read_max_bytes,
            lifecycle: Arc::new(Lifecycle::new(resources, workdir.clone())),
            pid,
            home: base,
            ws,
            workdir,
            storage: Arc::new(areas),
            tokens,
            supervisor: sup,
            scheduler,
            factory,
            results,
            sources: src,
            state,
            stats: stat,
        })
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    /// The checked-in HCL deserializes with the current version and settings.
    #[test]
    fn checked_in_hcl_matches_engine_schema() -> Result<()> {
        let config: EngineConfig = hcl::from_str(include_str!("engine.hcl"))?;
        assert_eq!(config.version(), ENGINE_VERSION);
        assert_eq!(
            config.runtime_settings(EngineMode::Daemon)?,
            (2, SYNC, TICK)
        );
        assert_eq!(
            config.runtime_settings(EngineMode::Single)?,
            (1, SYNC, Duration::from_secs(1))
        );
        assert_eq!(
            config.pipeline_queue_capacity,
            Some(DEFAULT_PIPELINE_QUEUE_CAPACITY)
        );
        assert_eq!(
            config.read_max_bytes,
            Some(producer::DEFAULT_READ_MAX_BYTES)
        );
        assert!(matches!(
            config.storage["hourly"],
            StorageConfig::Directory { .. }
        ));
        assert!(matches!(config.storage["hive"], StorageConfig::Hive { .. }));
        assert!(matches!(
            config.storage["cache"],
            StorageConfig::Cache { .. }
        ));
        Ok(())
    }

    /// Zero worker counts and timer intervals fail before the scheduler starts.
    #[rstest::rstest]
    #[case("workers = 0", "workers")]
    #[case("tick = { secs = 0, nanos = 0 }", "tick")]
    #[case("sync = { secs = 0, nanos = 0 }", "sync")]
    fn rejects_zero_runtime_settings(#[case] field: &str, #[case] name: &str) -> Result<()> {
        let input = format!("version = 3\nbasedir = \".\"\nstorage = {{}}\n{field}");
        let config: EngineConfig = hcl::from_str(&input)?;
        let error = config.runtime_settings(EngineMode::Daemon).unwrap_err();
        let field = match error.downcast_ref::<EngineStatus>() {
            Some(EngineStatus::ZeroWorkers) => "workers",
            Some(EngineStatus::ZeroInterval(field)) => field,
            _ => panic!("unexpected error: {error:?}"),
        };
        assert_eq!(field, name);
        Ok(())
    }
}
