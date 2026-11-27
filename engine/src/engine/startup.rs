//! Configuration loading, actor startup, and rollback for a new engine instance.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use eyre::Result;
use futures_util::StreamExt;
use object_store::local::LocalFileSystem;
use object_store::{ObjectStore, ObjectStoreExt};
use ractor::factory::{Factory, FactoryArguments, queues, routing};
use ractor::registry::registered;
use ractor::{Actor, call, cast};
use tokio::fs;
use tracing::{error, info, trace};

use fetiche_common::{ConfigFile, Versioned};

use crate::actors::*;
use crate::config::{DEFAULT_PIPELINE_QUEUE_CAPACITY, ENGINE_CONFIG, ENGINE_VERSION};
use crate::{EngineConfig, EngineStatus, Storage, Workspace, producer};

use super::lifecycle::{Lifecycle, Resources};
use super::{Engine, EngineMode};

/// File recording the engine process ID.
const ENGINE_PID: &str = "acutectl.pid";

static NEXT_ENGINE_INSTANCE: AtomicUsize = AtomicUsize::new(0);

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
        // BASEDIR/var/run/acute for daemon mode
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
                #[cfg(feature = "asd")]
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
