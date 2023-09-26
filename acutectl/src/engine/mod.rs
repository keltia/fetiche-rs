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
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;
use std::{fs, thread};

use eyre::Result;
#[cfg(unix)]
use home::home_dir;
use serde::Deserialize;
use tracing::{debug, error, event, info, trace, warn, Level};

pub use config::*;
pub use database::*;
pub use fetiche_formats::Format;
pub use fetiche_sources::{makepath, Auth, Fetchable, Filter, Flow, Site, Sources, Streamable};
pub use job::*;
pub use parse::*;
pub use state::*;
pub use storage::*;
use strum::EnumString;
pub use task::*;

mod config;
mod database;
mod job;
mod parse;
mod state;
mod storage;
mod task;

/// Engine signature
///
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Configuration filename
const ENGINE_CONFIG: &str = "engine.hcl";

/// FIXME: Current running process ID — this will be handled by `fetiched` when operational
const ENGINE_PID: &str = "fetiched.pid";

/// Configuration file version
const ENGINE_VERSION: usize = 2;

/// Main state data file, will be created in `basedir`.
pub(crate) const STATE_FILE: &str = "state";

/// Tick is every 30s
const TICK: u64 = 30;

/// An `Engine` instance has a command channel for commands.
///
#[derive(Clone, Debug, strum::Display, EnumString)]
pub enum EngineCtrl {
    Start,
    Stop,
    Sync,
    List,
}

/// Main `Engine` struct that hold the sources and everything needed to perform
///
#[derive(Clone, Debug)]
pub struct Engine {
    /// Command channel
    pub ctrl: Sender<EngineCtrl>,
    /// Current process DI
    pub pid: u32,
    /// Next job ID
    pub next: Arc<AtomicUsize>,
    /// Main area where state is saved (PID, jobs, etc.)
    pub home: Arc<PathBuf>,
    /// Sources
    pub sources: Arc<Sources>,
    /// Storage area for long running jobs
    pub storage: Arc<Storage>,
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
        Self::load(Self::default_file()).clone()
    }

    /// Load configuration file for the engine.
    ///
    /// Takes a string or anything that can be turned into a `PathBuf`.
    ///
    #[tracing::instrument]
    pub fn load<T>(fname: T) -> Self
    where
        T: Into<PathBuf> + Debug,
    {
        let fname = fname.into();

        trace!("reading({:?}", fname);

        let data =
            fs::read_to_string(&fname).unwrap_or_else(|_| panic!("file not found {:?}", fname));

        let cfg: EngineConfig = hcl::from_str(&data).expect("syntax error");

        // Bail out if different
        //
        if cfg.version != ENGINE_VERSION {
            event!(
                Level::ERROR,
                tag = "bad config version",
                version = cfg.version
            );
            panic!(
                "Only v{} config file supported in {}",
                ENGINE_VERSION,
                fname.to_string_lossy()
            );
        }
        Self::from_cfg(&cfg).clone()
    }

    /// Create a new instance from a EngineConfig struct
    ///
    /// FIXME: too many paths hard-coded in the `engine.hcl` or `storage.hcl` files.
    ///
    #[tracing::instrument]
    pub fn from_cfg(cfg: &EngineConfig) -> Self {
        trace!("load sources");

        // Register sources
        //
        let src = match Sources::load(&None) {
            Ok(src) => src,
            Err(e) => panic!("No sources configured in 'sources.hcl':{}", e),
        };
        info!("{} sources loaded", src.len());

        trace!("load storage areas");
        // Register storage areas
        //
        let areas = Storage::register(&cfg.storage);
        info!("{} areas loaded", areas.len());

        // Save PID
        //
        let pid = std::process::id();
        let basedir: PathBuf = cfg.basedir.clone();
        let pidfile: PathBuf = makepath!(&basedir, ENGINE_PID);
        fs::write(&pidfile, format!("{pid}")).expect("can not write fetiched.pid");

        info!("PID {} written in {:?}", pid, pidfile);

        // Load state
        //
        let fname: PathBuf = makepath!(&basedir, STATE_FILE);
        let state = match State::from(fname.clone()) {
            Ok(state) => {
                info!("State loaded from {:?}", fname);
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
        // Control channel first
        //
        let (tx, rx) = channel::<EngineCtrl>();

        // tx is not in an Arc because it is clonable
        //
        let mut engine = Engine {
            ctrl: tx,
            pid,
            next: Arc::new(AtomicUsize::new(state.last + 1)),
            home: Arc::new(basedir),
            sources: Arc::new(src),
            storage: Arc::new(areas),
            state: Arc::new(RwLock::new(state)),
            jobs: Arc::new(RwLock::new(jobs)),
        };
        info!("New Engine loaded");

        // Sync immediately, ensuring state is clean
        //
        engine.sync().expect("can not sync");

        // Launch the control channel thread
        //
        trace!("launching controller");

        let e = engine.clone();
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                trace!("engine::controller: command: {}", msg);

                match msg {
                    EngineCtrl::Sync => match e.sync() {
                        Ok(_) => (),
                        Err(e) => error!("engine::controller: sync failed: {}", e.to_string()),
                    },
                    _ => (),
                };
            }
        });

        // Launch the sync thread for state
        //
        trace!(
            "launching syncer for {}, every {}s",
            engine.state_file().to_string_lossy(),
            TICK
        );

        let e = engine.clone();
        thread::spawn(move || loop {
            if let Err(err) = e.command(EngineCtrl::Sync) {
                error!("engine::sync failed: {}", err.to_string());
            }
            thread::sleep(Duration::from_secs(TICK));
        });

        engine
    }

    /// Send a command to the engine
    ///
    #[tracing::instrument(skip(self))]
    pub fn command(&self, cmd: EngineCtrl) -> Result<()> {
        Ok(self.ctrl.send(cmd)?)
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

        trace!("create_job with id: {}", nextid);
        self.command(EngineCtrl::Sync).expect("can not sync");

        job
    }

    /// Remove a job
    ///
    #[tracing::instrument]
    pub fn remove_job(&mut self, job: Job) -> Result<()> {
        trace!("grab lock");

        let mut state = self.state.try_write().unwrap();
        state.remove_job(job.id);

        // Prevent deadlock by dropping ownership here, must be a better way to handle this
        //
        drop(state);

        trace!("sync");
        Ok(self.sync()?)
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
        let def: PathBuf = makepath!(homedir, BASEDIR, "drone-utils");
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