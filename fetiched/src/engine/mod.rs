//! Library implementing common part of the transformations
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
use std::convert::Into;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use actix::prelude::*;
use directories::{BaseDirs, ProjectDirs};
use eyre::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::EnumString;
use tracing::{debug, error, info, trace, warn};

pub use error::*;
use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_formats::Format;
use fetiche_macros::into_configfile;
pub use fetiche_sources::{Auth, Fetchable, Filter, Flow, Site, Sources, Streamable};
pub use job::*;
pub use parse::*;
//pub use state::*;
pub use task::*;

use crate::{
    Bus, ConfigActor, GetState, StateActor, StorageActor, StorageConfig, StorageList, Sync, System,
    UpdateState,
};

mod error;
mod job;
mod parse;
//mod state;
mod task;

/// Engine signature
///
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// Configuration filename
const ENGINE_CONFIG: &str = "engine.hcl";

/// File containing known sources
///
const SOURCES_CONFIG: &str = "sources.hcl";

/// Configuration file version
const ENGINE_VERSION: usize = 2;

/// Tick is every 30s
const TICK: u64 = 30;

/// Configuration file format
#[into_configfile(version = 2, filename = "engine.hcl")]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct EngineConfig {
    /// Base directory
    pub basedir: PathBuf,
    /// List of storage types
    pub storage: BTreeMap<String, StorageConfig>,
}

/// Main `Engine` struct that hold the sources and everything needed to perform
///
#[derive(Clone, Debug)]
pub struct Engine {
    /// Addr of `ConfigActor`
    config: Addr<ConfigActor>,
    /// State management agent
    pub state: Addr<StateActor>,
    /// Storage area for long running jobs
    pub store: Addr<StorageActor>,
    //
    /// Next job ID
    pub next: Arc<AtomicUsize>,
    /// Main area where state is saved (PID, jobs, etc.)
    pub home: Arc<PathBuf>,
    /// Sources
    pub sources: Arc<Sources>,
    /// Job Queue
    pub jobs: Arc<RwLock<VecDeque<usize>>>,
}

/// This is the struct that gets sent over to the state actor for sync.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
struct EngineState {
    /// Next job ID
    pub next: usize,
    /// Job Queue
    pub jobs: VecDeque<usize>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            next: 0,
            jobs: VecDeque::new(),
        }
    }
}

impl Engine {
    /// Create an instance
    ///
    #[tracing::instrument(skip(bus))]
    pub async fn new(workdir: &PathBuf, bus: &Bus) -> Self {
        trace!("new engine({:?})", workdir);

        let state = bus.state.clone();
        let store = bus.store.clone();
        let config = bus.config.clone();

        let fname = ENGINE_CONFIG;
        let root = ConfigFile::<EngineConfig>::load(Some(fname))?;
        let cfg = root.inner();
        let home = root.config_path();
        trace!("Home is in {home:?}");

        // Bail out if different
        //
        if cfg.version() != ENGINE_VERSION {
            error!("Bad config version {}", cfg.version());
            panic!("Bad config version {}", cfg.version())
        }

        // Register sources
        //
        trace!("load sources");
        let src = Sources::new()?;
        info!("{} sources loaded", src.len());

        trace!("loading state");
        let ourstate = if let Ok(state) = state.send(GetState::about(System::Engine)).await {
            match serde_json::from_str(&state) {
                Ok(state) => {
                    info!("state loaded.");
                    state
                }
                _ => {
                    warn!("empty state");
                    EngineState::default()
                }
            }
        } else {
            EngineState::default()
        };
        debug!("engine={:?}", ourstate);

        state.do_send(Sync);

        trace!("load storage areas");
        // Register storage areas
        //
        if let Ok(areas) = store.send(StorageList).await.unwrap() {
            info!("{} areas loaded.", areas.len());
        }

        let jobs = VecDeque::<usize>::new();

        // Instantiate everything
        //
        let engine = Engine {
            config,
            state,
            store,
            next: Arc::new(AtomicUsize::new(ourstate.next)),
            home: Arc::new(workdir.clone()),
            sources: Arc::new(src),
            jobs: Arc::new(RwLock::new(jobs)),
        };
        info!("New Engine loaded");

        // Sync immediately, ensuring state is clean
        //
        let _ = engine
            .state
            .send(UpdateState::service(
                System::Engine,
                json!(ourstate).to_string(),
            ))
            .await
            .expect("can not UpdateState");

        engine
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

        // Update state
        //
        let state = EngineState {
            next: self.next.load(Ordering::SeqCst),
            jobs: jobs.clone(),
        };
        let state = json!(state).to_string();

        // Ensure lock goes away
        //
        drop(jobs);

        trace!("create_job with id: {}", nextid);

        self.state
            .do_send(UpdateState::service(System::Engine, state));

        job
    }

    /// Remove a job
    ///
    #[tracing::instrument(skip(self))]
    pub fn remove_job(&mut self, job: Job) -> Result<()> {
        trace!("grab lock");

        // Find the job in queue, remove it and update our state
        //
        let mut jobs = self.jobs.try_write().unwrap();
        let ind = jobs.binary_search(&job.id).unwrap();
        jobs.remove(ind);

        // Update state
        //
        let state = EngineState {
            next: self.next.load(Ordering::Relaxed),
            jobs: jobs.clone(),
        };
        let state = json!(state).to_string();
        debug!("{}", state);

        // Prevent deadlock by dropping ownership here, must be a better way to handle this
        //
        drop(jobs);

        trace!("sync");
        Ok(self
            .state
            .do_send(UpdateState::service(System::Engine, state)))
    }

    /// Returns the path of the default config directory
    ///
    #[cfg(unix)]
    pub fn config_path() -> PathBuf {
        let homedir = match BaseDirs::new() {
            Some(dirs) => dirs.home_dir(),
            None => std::env::var("HOME").unwrap().into(),
        };
        homedir.join(".config").join("drone-utils")
    }

    /// Returns the path of the default config directory
    ///
    #[cfg(windows)]
    pub fn config_path() -> PathBuf {
        let basedir = match BaseDirs::new() {
            Some(dirs) => dirs.data_local_dir(),
            None => std::env::var("LOCALAPPDATA").unwrap().into(),
        };

        Path::new(basedir).join("drone-utils")
    }

    /// Returns the path of the default config file
    ///
    pub fn default_file() -> PathBuf {
        Self::config_path().join(ENGINE_CONFIG)
    }

    /// Return an `Arc::clone` of the Engine sources
    ///
    pub fn sources(&self) -> Arc<Sources> {
        Arc::clone(&self.sources)
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

    /// Return a list of all currently available authentication tokens
    ///
    pub fn list_tokens(&self) -> Result<String> {
        self.list_tokens()
    }

    /// Return Engine version (and internal modules)
    ///
    pub fn version(&self) -> String {
        format!(
            "{} ({})",
            version(),
            fetiche_formats::version(),
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
    /// Both (middle)
    #[default]
    Filter,
    /// Cache (middle)
    Cache,
}

/// Anything that can be `run()` is runnable.
///
/// See the engine-macro crate for a proc-macro that implement the `run()`  wrapper for
/// the `Runnable` trait.
///
pub trait Runnable: Debug {
    fn cap(&self) -> IO;
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}
