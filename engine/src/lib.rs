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

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use eyre::Result;
use serde::Deserialize;
use strum::EnumString;
use tracing::{debug, error, info, trace, warn};

use fetiche_common::{ConfigFile, Container, IntoConfig, Versioned};
use fetiche_formats::Format;
use fetiche_macros::into_configfile;
use fetiche_sources::Sources;

pub use error::*;
pub use job::*;
pub use parse::*;
pub use state::*;
pub use storage::*;
pub use task::*;
pub use tokens::*;

mod error;
mod job;
mod parse;
mod state;
mod storage;
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

/// Main state data file, will be created in `basedir`.
pub(crate) const STATE_FILE: &str = "state";

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

/// Main `Engine` struct that hold the sources and everything needed to perform
///
#[derive(Clone, Debug)]
pub struct Engine {
    /// Current process DI
    pub pid: u32,
    /// Next job ID
    pub next: Arc<AtomicUsize>,
    /// Main area where state is saved (PID, jobs, etc.)
    pub home: Arc<PathBuf>,
    /// Sources
    pub sources: Arc<Sources>,
    /// Storage area for long-running jobs
    pub storage: Arc<Storage>,
    /// Storage are for auth tokens
    pub tokens: Arc<TokenStorage>,
    /// Current state
    pub state: Arc<RwLock<State>>,
    /// Job Queue
    pub jobs: Arc<RwLock<VecDeque<usize>>>,
}

impl Engine {
    /// Create an instance
    ///
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("new engine");

        // Load storage areas from `engine.hcl`
        //
        Self::load(ENGINE_CONFIG).unwrap_or_else(|e| {
            error!("Can not create Engine: {}", e.to_string());
            panic!("Error: {}", e.to_string())
        })
    }

    /// Load configuration file for the engine.
    ///
    /// Takes a string or anything that can be turned into a `PathBuf`.
    ///
    #[tracing::instrument]
    pub fn load(fname: &str) -> Result<Self> {
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

        trace!("load sources");
        let src = Sources::load()?;
        info!("{} sources loaded", src.len());

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

        // Save PID
        //
        let pid = std::process::id();
        let pidfile = home.join(ENGINE_PID);
        fs::write(&pidfile, format!("{pid}"))
            .unwrap_or_else(|_| panic!("can not write {}", pidfile.to_string_lossy()));

        info!("PID {} written in {:?}", pid, pidfile);

        // Load state
        //
        let fname = home.join(STATE_FILE);
        let state = match State::from(fname.clone()) {
            Ok(state) => {
                info!("State loaded from {}", fname.to_string_lossy());
                debug!("{:?}", state);
                state
            }
            Err(e) => {
                warn!("Can not load state, creating new: {}", e.to_string());
                State::new()
            }
        };
        trace!("state={:?}", state);

        let jobs = VecDeque::<usize>::new();

        // Instantiate everything
        //
        let engine = Engine {
            pid,
            next: Arc::new(AtomicUsize::new(state.last + 1)),
            home: Arc::new(home.clone()),
            sources: Arc::new(src.clone()),
            storage: Arc::new(areas),
            tokens: Arc::new(tokens),
            state: Arc::new(RwLock::new(state)),
            jobs: Arc::new(RwLock::new(jobs)),
        };
        info!("New Engine loaded");

        // Sync immediately, ensuring state is clean
        //
        engine.sync().expect("can not sync");

        Ok(engine)
    }

    /// Create a new job queue
    ///
    #[tracing::instrument(skip(self))]
    pub fn create_job(&mut self, s: &str) -> Job {
        // Fetch next ID
        //
        let nextid = self.next.fetch_add(1, Ordering::SeqCst);

        // Initialise job
        //
        let job = Job::new_with_id(s, nextid);

        // Insert into job queue
        //
        let mut jobs = self.jobs.write().unwrap();
        jobs.push_back(nextid);

        // Ensure lock goes away
        //
        drop(jobs);

        // Update state
        //
        let mut state = self.state.write().unwrap();
        state.last = nextid;
        state.queue.push_back(nextid);

        // Ensure lock goes away
        //
        drop(state);

        trace!("job {} created.", nextid);
        self.sync().expect("can not sync");

        job
    }

    /// Remove a job
    ///
    #[tracing::instrument(skip(self))]
    pub fn remove_job(&mut self, job: Job) -> Result<()> {
        trace!("grab lock");

        let mut state = self.state.try_write().unwrap();
        state.remove_job(job.id);

        // Prevent deadlock by dropping ownership here, must be a better way to handle this
        //
        drop(state);

        trace!("sync");
        self.sync()
    }

    /// Return an `Arc::clone` of the Engine sources
    ///
    pub fn sources(&self) -> Arc<Sources> {
        Arc::clone(&self.sources)
    }

    /// Return an `Arc::clone` of the Engine storage areas
    ///
    pub fn storage(&self) -> Arc<Storage> {
        Arc::clone(&self.storage)
    }

    /// Returns a list of all defined storage areas
    ///
    pub fn list_storage(&self) -> Result<String> {
        self.storage.list()
    }

    /// Return a description of all supported sources
    ///
    pub fn list_sources(&self) -> Result<String> {
        self.sources.list()
    }

    /// Return a descriptions of all supported data formats
    ///
    pub fn list_formats(&self) -> Result<String> {
        Format::list()
    }

    /// Return a descriptions of all supported container formats
    ///
    pub fn list_containers(&self) -> Result<String> {
        Container::list()
    }

    /// Return a list of all currently available authentication tokens
    ///
    pub fn list_tokens(&self) -> Result<String> {
        self.tokens.list()
    }

    /// Return Engine version (and internal modules)
    ///
    pub fn version(&self) -> String {
        format!(
            "{} ({} {} {})",
            version(),
            fetiche_formats::version(),
            fetiche_sources::version(),
            fetiche_common::version(),
        )
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
pub trait Runnable: Debug {
    fn cap(&self) -> IO;
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}
