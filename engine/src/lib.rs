//! Library implementing common part of the transformations
//!
//! This is a *synchronous* engine.  It needs to be called in a synchronous context even if the
//! program calling it is async.
//!
//! Example:
//! ```no_run
//! # async fn main() {
//! use tracing::trace;
//! use fetiche_engine::Engine;
//!
//! // Instantiate Engine
//! //
//! let engine = Engine::new();
//! trace!("Engine initialised and running.");
//!
//! // For the moment the whole of Engine is sync so we need to block.
//! //
//! let res = tokio::task::spawn_blocking(move || println!("{}", engine.list_tokens())).await?;
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
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use enum_dispatch::enum_dispatch;
use eyre::Result;
use ractor::{call, cast, Actor, ActorRef};
use serde::Deserialize;
use strum::EnumString;
use tracing::{error, info, trace, warn};

use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;

//pub use parse::*;
pub use error::*;
pub use job::*;
pub use queue::*;
//pub use state::*;
pub use storage::*;
pub use task::*;
pub use tokens::*;

use crate::actors::*;

mod actors;
mod error;
mod job;
//mod parse;
mod queue;
//mod state;
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
const ENGINE_VERSION: usize = 2;

/// Engine process group for the actors
const ENGINE_PG: &str = "engine.pg";

/// Configuration file format
#[into_configfile(version = 2, filename = "engine.hcl")]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct EngineConfig {
    /// Base directory
    pub basedir: PathBuf,
    /// List of storage types
    pub storage: BTreeMap<String, StorageConfig>,
}

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
/// - `next`
///     The next job ID to be used. Tracked for managing job identifiers.
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
    /// Next job ID
    pub next: usize,
    /// Main area where state is saved (PID, jobs, etc.)
    pub home: Arc<PathBuf>,
    /// Sources
    pub sources: ActorRef<SourcesMsg>,
    /// Storage area for long-running jobs
    pub storage: Arc<Storage>,
    /// Storage are for auth tokens
    pub tokens: Arc<TokenStorage>,
    /// Current state
    pub state: ActorRef<StateMsg>,
    /// Job Queue
    pub jobs: Arc<RwLock<JobQueue>>,
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

    /// Load configuration file for the engine.
    ///
    /// Takes a string or anything that can be turned into a `PathBuf`.
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
        //

        // Start sources service
        //
        trace!("load sources");
        let (src, _h) = Actor::spawn(Some("engine::sources".into()), SourcesActor, ()).await?;
        let count = call!(src, |port| SourcesMsg::Count(port))?;
        info!("{} sources loaded", count);

        // Start state service
        //
        trace!("load state.");
        let (state, _h) =
            Actor::spawn(Some("engine::state".into()), StateActor, home.clone()).await?;
        trace!("state={:?}", state);

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

        // Sync is every 30s
        //
        let _ = state.send_interval(Duration::from_secs(30), || StateMsg::Sync);

        let jobs = JobQueue::new();
        let last = call!(state, |port| StateMsg::Next(port))?;

        // Instantiate everything
        //
        let engine = Engine {
            pid,
            next: last,
            home: Arc::new(home),
            sources: src.clone(),
            storage: Arc::new(areas),
            tokens: Arc::new(tokens),
            state: state.clone(),
            jobs: Arc::new(RwLock::new(jobs)),
        };
        info!("New Engine loaded pid={}", pid);

        // Sync immediately, ensuring state is clean
        //
        let _ = engine.sync()?;

        Ok(engine)
    }

    /// Create a new job queue
    ///
    #[tracing::instrument(skip(self))]
    pub async fn create_job(&mut self, s: &str) -> Result<Job> {
        // Fetch next ID
        //
        let nextid = call!(self.state, |port| StateMsg::Next(port))?;

        // Initialise job
        //
        let job = Job::new_with_id(s, nextid);

        // Insert into job queue
        //
        let mut jobs = self.jobs.write().unwrap();
        jobs.add(job.clone());
        drop(jobs);

        // Update state
        //
        let _ = cast!(self.state, StateMsg::Add(nextid))?;

        trace!("job {} created.", nextid);
        self.sync()?;

        Ok(job)
    }

    /// Remove a job
    ///
    #[tracing::instrument(skip(self))]
    pub fn remove_job(&mut self, job: Job) -> Result<()> {
        trace!("grab lock");

        let _ = cast!(self.state, StateMsg::Remove(job.id))?;

        trace!("sync");
        self.sync()
    }

    #[tracing::instrument(skip(self))]
    pub fn get_job(&self, id: usize) -> Result<Job> {
        let state = self.jobs.read().unwrap();
        let job = match state.get(id) {
            Some(job) => job,
            None => {
                return Err(EngineStatus::JobNotFound(id).into());
            }
        };
        Ok(job.clone())
    }
}

/// Task I/O characteristics
///
/// The main principle being that a consumer should not be first in a job queue
/// just like an Out one should not be last.
///
#[derive(Clone, Debug, Default, Eq, PartialEq, EnumString, strum::Display, Deserialize)]
#[strum(serialize_all = "PascalCase")]
pub enum IO {
    /// Consumer (no output or different like file)
    Consumer,
    /// Producer (discard input)
    Producer,
    /// Both (filter)
    #[default]
    Filter,
    /// Cache (filter)
    Cache,
}

/// Anything that can be `run()` is runnable.
///
/// See the engine-macro crate for a proc-macro that implement the `run()`  wrapper for
/// the `Runnable` trait.
///
/// ```no_run
/// use fetiche_engine::{IO, Runnable};
/// use fetiche_formats::Format;
/// use fetiche_macros::RunnableDerive;
///
/// #[derive(Clone, Debug, RunnableDerive)]
/// pub struct Convert {
///     io: IO,
///     pub from: Format,
///     pub into: Format,
/// }
/// ```
///
///
#[enum_dispatch(Task)]
pub trait Runnable: Debug {
    fn cap(&self) -> IO;
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}
