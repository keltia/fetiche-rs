//! Library implementing common part of the transformations
//!
//! # Config
//!
//! v2: Has basedir and storage components.
//! v2: Has initial number of workers for the runner factory.
//!
//! Example:
//! ```no_run
//! # #[tokio::main]
//! # async fn main() -> eyre::Result<()> {
//! use tracing::trace;
//! use fetiche_engine::Engine;
//!
//! // Instantiate Engine
//! //
//! let engine = Engine::new().await;
//! trace!("Engine initialised and running.");
//!
//! // For the moment the whole of Engine is sync so we need to block.
//! //
//! let res = tokio::task::spawn_blocking(move || println!("{}", engine.list_tokens().unwrap())).await?;
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
//! FIXME: at some point, a `[u8]`  might be preferable to a `String`.
//!
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use enum_dispatch::enum_dispatch;
use eyre::Result;
use ractor::factory::{queues, routing, Factory, FactoryArguments};
use ractor::{call, cast, Actor, ActorRef};
use serde::Deserialize;
use strum::EnumString;
use tracing::{error, info, trace, warn};

use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;

pub use error::*;
pub use filter::*;
pub use job::*;
pub use parse::*;
pub use producer::*;
pub use storage::*;
pub use task::*;
pub use tokens::*;

use crate::actors::*;

mod actors;
mod error;
mod filter;
mod job;
mod parse;
mod producer;
mod storage;
mod subr;
mod task;
mod tokens;
mod consumer;

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
const ENGINE_VERSION: usize = 2;

/// Engine process group for the actors
const ENGINE_PG: &str = "engine.pg";

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
    pub workers: usize,
    /// List of storage types
    pub storage: BTreeMap<String, StorageConfig>,
}

/// The `StorageConfig` enum defines different storage types supported by the engine.
/// It allows the engine to specify and configure storage modules based on the operational
/// requirements (e.g., in-memory caching, local filesystem storage, or Hive-based sharding).
///
/// # Variants
///
/// - `Cache`
///     Defines an in-memory key-value store configuration, typically connected to a service
///     like DragonflyDB or REDIS. Requires a `url` to connect.
///
/// - `Directory`
///     Represents storage based on the local filesystem. Includes a `path` to the directory
///     and a `rotation` mechanism for maintaining storage consistency or archival.
///
/// - `Hive`
///     Adds support for Hive-based sharding. Designed for scalable and distributed storage.
///     Includes a `path` for file-based Hive shards.
///
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum StorageConfig {
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache { url: String },
    /// In the local filesystem
    Directory { path: PathBuf, rotation: String },
    /// HIVE-based sharding
    Hive { path: PathBuf },
}

/// The `Engine` struct is the main structure for coordinating all tasks, jobs, storage,
/// and actors within the application. It provides functionality for managing the runtime
/// environment, including sources, storage, token management, state synchronization,
/// and job queue management.
///
/// # Fields
///
/// - `pid`
///     The current process ID for the engine. Retrieved from the state service.
///
/// - `last`
///     The last used ID. Tracked for managing job identifiers.
///
/// - `home`
///     The root directory where state is saved. This directory includes the configuration file,
///     PID file, and other engine-related paths.
///
/// - `sources`
///     An actor reference for the sources service. Handles loading and management of source objects.
///
/// - `storage`
///     A reference to the `Storage` struct that handles storage areas for long-running jobs.
///
/// - `tokens`
///     A reference to the `TokenStorage` struct that manages authentication tokens used by the engine.
///
/// - `state`
///     An actor reference for the state service, which manages the engine's internal state,
///     including synchronization and saving runtime information.
///
/// - `jobs`
///     A thread-safe, read-write lock to the `JobQueue`, which maintains a pipeline of tasks and jobs.
///
/// # Usage
///
/// The `Engine` struct can be instantiated using either the `new` method or by loading
/// a configuration file using the `load` method. It integrates with other components of the
/// system through actors and performs various initialization routines.
///
/// Example:
///
/// ```no_run
/// # use tokio;
/// # use fetiche_engine::Engine;
/// #[tokio::main]
/// async fn main() {
///     // Initialize a new engine asynchronously
///     let engine = Engine::new().await;
///     println!("Engine initialized with PID: {}", engine.pid);
/// }
/// ```
///
/// The engine ensures proper initialization of all necessary services, such as source loading,
/// state synchronization, and storage registration, during its setup process.
///
#[derive(Clone, Debug)]
pub struct Engine {
    /// Current process DI
    pub pid: u32,
    /// Last used job ID
    pub last: usize,
    /// Main area where state is saved (PID, jobs, etc.)
    pub home: Arc<PathBuf>,
    /// Job Queue actor
    pub queue: ActorRef<QueueMsg>,
    /// Sources
    pub sources: ActorRef<SourcesMsg>,
    /// Storage area for long-running jobs
    pub storage: Arc<Storage>,
    /// Storage are for auth tokens
    pub tokens: Arc<TokenStorage>,
    /// Current state
    pub state: ActorRef<StateMsg>,
    /// Stats gathering actors for sources
    pub stats: ActorRef<StatsMsg>,
    /// Supervisor actor, top of the process group
    pub supervisor: ActorRef<SuperMsg>,
}

impl Engine {
    /// Create an instance
    ///
    #[tracing::instrument]
    pub async fn new() -> Self {
        trace!("new engine");

        // Load storage areas from `engine.hcl`
        //
        Self::load(ENGINE_CONFIG).await.unwrap_or_else(|e| {
            error!("Can not create Engine: {}", e.to_string());
            panic!("Error: {e}")
        })
    }

    /// Load an engine configuration file and initialize the `Engine`.
    ///
    /// This method reads the specified configuration file, validates its version, and initializes
    /// the required engine components like actors, state, storage, and token management. It also
    /// ensures that the `Engine` syncs its runtime state upon creation to maintain consistency.
    ///
    /// # Parameters
    ///
    /// - `fname`: A string slice representing the path to the configuration file.
    ///
    /// # Returns
    ///
    /// - On success, returns an instance of the `Engine` struct initialized with the provided configuration.
    /// - On failure, returns an `Err` containing details about the error.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use fetiche_engine::Engine;
    /// # use tokio;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let engine = Engine::new().await;
    ///     println!("Engine PID: {}", engine.pid);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// The function will return an error in the following cases:
    /// - If the configuration file cannot be loaded or parsed.
    /// - If the configuration version does not match the expected `ENGINE_VERSION`.
    /// - If any of the actors fail to spawn or initialize correctly.
    ///
    /// # Tracing
    /// This method uses tracing to log detailed events during execution, including loading sources,
    /// initializing storage, syncing state, and handling errors. Ensure tracing is set up correctly
    /// to observe these events.
    ///
    #[tracing::instrument]
    pub async fn load(fname: &str) -> Result<Self> {
        trace!("reading({:?}", fname);

        let root = ConfigFile::<EngineConfig>::load(Some(fname))?;
        let cfg = root.inner();
        let home = root.config_path();
        trace!("Home is in {home:?}");

        // Bail out if different
        //
        if cfg.version() != ENGINE_VERSION {
            error!("Bad config version {}", cfg.version());
            return Err(EngineStatus::BadConfigVersion(cfg.version(), ENGINE_VERSION).into());
        }

        // ----- Start actors

        // We have a generic supervisor actor.
        //
        trace!("starting supervisor actor.");
        let tag = String::from("sources:supervisor");
        let (sup, _h) = Actor::spawn(Some(tag), Supervisor, ()).await.unwrap();

        // Start the stats gathering service.
        //
        trace!("starting stats actor.");
        let tag = String::from("sources::stats");
        let (stat, _h) = Actor::spawn_linked(
            Some(tag),
            StatsActor,
            "sources".into(),
            sup.get_cell())
            .await?;

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

        let count = call!(src, |port| SourcesMsg::Count(port))?;
        info!("{} sources loaded", count);

        // Start state service
        //
        trace!("load state.");
        let (state, _h) = Actor::spawn_linked(
            Some("engine::state".into()),
            StateActor,
            home.clone(),
            sup.get_cell(),
        )
            .await?;
        trace!("state={:?}", state);

        // Get last used ID from previous state
        //
        let last = call!(state, |port| StateMsg::Last(port))?;

        // Start job queue service, upon startup the queue will always be empty.
        //
        trace!("load job queue");
        let (queue, _h) = Actor::spawn_linked(
            Some("engine::queue".into()),
            QueueActor,
            last,
            sup.get_cell(),
        )
            .await?;
        trace!("queue={:?}", queue);

        // ----- Start Runner Factory

        let factory_def = Factory::<
            (),
            RunnerMsg,
            (),
            RunnerActor,
            routing::QueuerRouting<(), RunnerMsg>,
            queues::DefaultQueue<(), RunnerMsg>,
        >::default();
        let factory_args = FactoryArguments::builder()
            .worker_builder(Box::new(RunnerBuilder))
            .queue(Default::default())
            .router(Default::default())
            .num_initial_workers(cfg.workers)
            .build();

        let (_factory, _h) = Actor::spawn(None, factory_def, factory_args).await?;

        // ----- Register non-actor sub-systems

        // Register storage areas
        //
        trace!("load storage areas");
        let areas = Storage::register(&cfg.storage);
        info!("{} areas loaded", areas.len());

        // Register tokens
        //
        trace!("load tokens");
        let tokens_area = cfg.basedir.join("tokens").to_string_lossy().to_string();
        let tokens = TokenStorage::register(&tokens_area);
        info!("{} tokens loaded", tokens.len());

        // Get PID from the state service
        //
        let pid = call!(state, |port| StateMsg::GetPid(port))?;

        // Instantiate everything
        //
        let engine = Engine {
            pid,
            last,
            home: Arc::new(home),
            queue: queue.clone(),
            sources: src.clone(),
            storage: Arc::new(areas),
            tokens: Arc::new(tokens),
            state: state.clone(),
            stats: stat.clone(),
            supervisor: sup.clone(),
        };
        info!("New Engine loaded pid={}", pid);

        // Sync immediately, ensuring state is clean
        //
        let _ = engine.sync()?;

        // If debug/trace, list all the actors running at this point.
        //
        let plist = registered().join('\n');
        trace!("Actor list: {plist}");
        Ok(engine)
    }

    ///
    ///
    /// Create a new job
    ///
    /// This method creates a new job within the engine by utilizing the internal job queue mechanism.
    /// The created job is assigned a unique ID, initialized, and synchronized with the engine state.
    ///
    /// # Arguments
    ///
    /// - `s`: A string slice representing the job's description or identifier.
    ///
    /// # Returns
    ///
    /// - On success, returns `Ok(Job)` with the created job.
    /// - On failure, returns an `Err` containing details about the error.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use fetiche_engine::{Engine, JobState};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut engine = Engine::new().await?;
    ///
    ///     let job = engine.create_job("example_job")?;
    ///     println!("Job created with ID: {}", job.id);
    ///     assert_eq!(job.state, JobState::Created);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error in the following cases:
    ///
    /// - If the job cannot be added to the queue.
    /// - If the state service fails to update for the new job.
    /// - If synchronization fails with the engine state after job creation.
    ///
    /// # Tracing
    ///
    /// Tracing logs provide insights during the job creation process:
    /// - Fetching the next job ID from the queue.
    /// - Initialization of the job.
    /// - Status of queue and state updates.
    /// - Synchronization with the engine state after job creation.
    ///
    /// Ensure tracing is properly configured in your application to monitor these events.
    ///
    #[tracing::instrument(skip(self))]
    pub fn create_job(&mut self, s: &str) -> Result<Job> {
        // Fetch next ID
        //
        let nextid = call!(self.queue, |port| QueueMsg::Allocate(port))?;

        // Initialise job, list of task is empty
        //
        let job = Job::new(s, nextid);

        // Update state
        //
        let _ = cast!(self.state, StateMsg::Add(nextid))?;

        trace!("job {} created.", nextid);
        self.sync()?;

        Ok(job)
    }

    /// Queue a job for execution in the engine.
    ///
    /// This method takes a job that is in the "Ready" state and queues it for execution
    /// by changing its state to "Queued" and adding it to the engine's job queue.
    ///
    /// # Arguments
    ///
    /// * `job` - The `Job` instance to be queued. The job must be in the `Ready` state.
    ///
    /// # Returns
    ///
    /// - On success, returns `Ok(usize)` containing the job's ID
    /// - On failure, returns an `Err` containing details about what went wrong
    ///
    /// # Errors
    ///
    /// This method will return an error in the following cases:
    ///
    /// - If the job is not in the `Ready` state (returns `EngineStatus::JobNotReady`)
    /// - If adding the job to the queue fails
    ///
    /// # Tracing
    ///
    /// This method is instrumented for tracing, excluding the `self` parameter.
    ///
    #[tracing::instrument(skip(self))]
    pub fn queue_job(&mut self, job: Job) -> Result<usize> {
        if job.state != JobState::Ready {
            error!("Job is not ready");
            return Err(EngineStatus::JobNotReady(job.id).into());
        }

        // Change status and insert the job into the queue.
        //
        let mut ready = job.clone();
        ready.state = JobState::Queued;
        let _ = cast!(self.queue, QueueMsg::Add(ready))?;
        Ok(job.id)
    }

    /// Removes a job from the engine by its ID.
    ///
    /// This method attempts to remove a job with the specified ID from the engine's job queue.
    /// The job cannot be removed if it is currently in the Running state.
    ///
    /// # Arguments
    ///
    /// - `job_id`: The unique identifier of the job to remove.
    ///
    /// # Returns
    ///
    /// - On success, returns `Ok(())` after removing the job and syncing state.
    /// - On failure, returns an `Err` containing details about what went wrong.
    ///
    /// # Errors
    ///
    /// This method will return an error in the following cases:
    ///
    /// - If the job is currently running (`EngineStatus::JobIsRunning`)
    /// - If the job cannot be found in the queue
    /// - If state synchronization fails after removal
    ///
    /// # Tracing
    ///
    /// This method is instrumented for tracing, excluding the `self` parameter.
    ///
    #[tracing::instrument(skip(self))]
    pub fn remove_job(&mut self, job_id: usize) -> Result<()> {
        let job = call!(self.queue, |port| QueueMsg::GetById(job_id, port))?;
        if job.state == JobState::Running {
            return Err(EngineStatus::JobIsRunning(job_id).into());
        }

        let _ = cast!(self.state, StateMsg::Remove(job_id))?;
        let _ = cast!(self.queue, QueueMsg::RemoveById(job_id))?;
        self.sync()
    }

    /// Retrieve a job by its unique ID
    ///
    /// This method takes a job ID (of type `usize`) and attempts to retrieve the
    /// corresponding job from the internal job queue. If a job with the specified ID
    /// exists, it is returned; otherwise, an error is generated indicating the job
    /// could not be found.
    ///
    /// # Arguments
    ///
    /// - `id`: A `usize` identifier representing the unique ID of the job to retrieve.
    ///
    /// # Returns
    ///
    /// - Returns the `Job` instance if it exists.
    /// - Returns an error if the job with the specified ID is not found.
    ///
    /// # Errors
    ///
    /// This method will return an `Err` containing `EngineStatus::JobNotFound` if
    /// the job does not exist in the internal job queue.
    ///
    /// # Tracing
    ///
    /// Tracing logs are emitted to provide detailed runtime diagnostics, including:
    /// - Lock acquisition on the job list.
    /// - Retrieval success or error cases.
    ///
    /// Ensure tracing is set up in your application to observe these events.
    ///
    #[tracing::instrument(skip(self))]
    pub fn get_job(&self, id: usize) -> Result<Job> {
        let job = call!(self.queue, |port| QueueMsg::GetById(id, port))?;

        Ok(job.clone())
    }

    /// Submits a new job to be executed by parsing the job string, setting it to Ready state,
    /// and queuing it for execution.
    ///
    /// # Parameters
    ///
    /// - `job_str`: A string slice containing the job description to be parsed into a `Job`.
    ///
    /// # Returns
    ///
    /// - Returns `Ok(usize)` containing the ID of the newly created and queued job.
    /// - Returns `Err` if job creation, parsing, queueing or state sync fails.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - Job string parsing fails
    /// - Job queueing fails
    /// - State synchronization fails
    ///
    /// # Notes
    ///
    /// The method performs the following steps:
    /// 1. Parses the job string into a Job struct
    /// 2. Sets the job state to Ready
    /// 3. Queues the job for execution
    /// 4. Synchronizes the engine state
    ///
    #[tracing::instrument(skip(self))]
    pub fn submit_job(&mut self, job_str: &str) -> Result<usize> {
        let mut job = self.parse(job_str)?;
        job.state = JobState::Ready;

        let job_id = self.queue_job(job)?;
        assert_eq!(job_id, job.id);
        self.sync()?;
        Ok(job_id)
    }
}

/// Anything that can be `run()` is runnable.
///
/// See the engine-macro crate for a proc-macro that implement the `run()`  wrapper for
/// the `Runnable` trait.
///
#[enum_dispatch(Task)]
pub trait Runnable: Debug {
    fn cap(&self) -> IO;
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}
