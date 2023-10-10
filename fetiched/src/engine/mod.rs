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
use eyre::Result;
#[cfg(unix)]
use home::home_dir;
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::EnumString;
use tracing::{debug, info, trace};

pub use database::*;
use fetiche_formats::Format;
pub use fetiche_sources::{makepath, Auth, Fetchable, Filter, Flow, Site, Sources, Streamable};
pub use job::*;
pub use parse::*;
//pub use state::*;
pub use task::*;

use crate::{Bus, ConfigActor, GetState, StateActor, StorageActor, StorageList, UpdateState};

mod database;
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

        trace!("loading sources");
        // Register sources
        //
        let sources = workdir.join(SOURCES_CONFIG);
        let src = match Sources::load(&Some(sources.clone())) {
            Ok(src) => src,
            Err(e) => panic!("No sources configured in '{:?}':{}", sources, e),
        };
        info!("{} sources loaded.", src.len());

        trace!("loading state");
        let ourstate = if let Ok(state) = state.send(GetState::about("engine")).await {
            info!("state loaded.");
            debug!("state={}", state);
            let s: EngineState = serde_json::from_str(&state).unwrap();
            s
        } else {
            EngineState::default()
        };
        debug!("engine={:?}", ourstate);

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
            .send(UpdateState::service("engine", json!(ourstate).to_string()))
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
            next: nextid,
            jobs: jobs.clone(),
        };
        let state = json!(state).to_string();

        // Ensure lock goes away
        //
        drop(jobs);

        trace!("create_job with id: {}", nextid);

        self.state.do_send(UpdateState::service("engine", state));

        job
    }

    /// Remove a job
    ///
    #[tracing::instrument]
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

        // Prevent deadlock by dropping ownership here, must be a better way to handle this
        //
        drop(jobs);

        trace!("sync");
        Ok(self.state.do_send(UpdateState::service("engine", state)))
    }

    /// Load authentication data
    ///
    #[tracing::instrument]
    pub fn auth(&mut self, db: BTreeMap<String, Auth>) -> &mut Self {
        // Generate a sources list with credentials
        //
        let mut srcs = BTreeMap::<String, Site>::new();

        self.sources.values().for_each(|site: &Site| {
            let mut s = site.clone();
            if let Some(auth) = db.get(&s.name().unwrap()) {
                s.auth(auth.clone());
            }
            let n = &s.name().unwrap();
            srcs.insert(n.clone(), s.clone());
        });
        self.sources = Arc::new(Sources::from(srcs));
        self
    }

    /// Returns the path of the default config directory
    ///
    #[cfg(unix)]
    pub fn config_path() -> PathBuf {
        let homedir = home_dir().unwrap();
        let def: PathBuf = makepath!(homedir, ".config", "drone-utils");
        def
    }

    /// Returns the path of the default config directory
    ///
    #[cfg(windows)]
    pub fn config_path() -> PathBuf {
        let homedir = env!("LOCALAPPDATA");

        let def: PathBuf = makepath!(homedir, "drone-utils");
        def
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
        self.sources.list_tokens()
    }

    /// Return Engine version (and internal modules)
    ///
    pub fn version(&self) -> String {
        format!(
            "{} ({} {})",
            version(),
            fetiche_formats::version(),
            fetiche_sources::version()
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
pub trait Runnable: Debug {
    fn cap(&self) -> IO;
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}
