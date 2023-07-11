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

use std::collections::BTreeMap;
use std::convert::Into;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::Result;
#[cfg(unix)]
use home::home_dir;
use serde::Deserialize;
use strum::EnumString;
use tracing::{event, info, trace, Level};

pub use config::*;
// Re-export formats/sources.
pub use fetiche_formats::Format;
pub use fetiche_sources::{makepath, Auth, Fetchable, Filter, Flow, Site, Sources, Streamable};
pub use job::*;
pub use storage::*;
pub use task::*;

mod config;
mod job;
mod parse;
mod storage;
mod task;

pub(crate) fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Configuration filename
const ENGINE_CONFIG: &str = "engine.hcl";

/// Current running process ID
const ENGINE_PID: &str = "fetiched.pid";

/// Configuration file version
const ENGINE_VERSION: usize = 2;

/// Main `Engine` struct that hold the sources and everything needed to perform
///
#[derive(Clone, Debug)]
pub struct Engine {
    /// Current process DI
    pub pid: u32,
    /// Main area where state is saved (PID, jobs, etc.)
    pub home: Arc<PathBuf>,
    /// Sources
    pub sources: Arc<Sources>,
    /// Storage area for long running jobs
    pub storage: Arc<Storage>,
}

impl Engine {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("new engine");
        // Load storage areas from `engine.hcl`
        //
        Self::with(Self::default_file())
    }

    // Load configuration file for storage areas
    //
    #[tracing::instrument]
    pub fn with<T>(fname: T) -> Self
    where
        T: Into<PathBuf> + Debug,
    {
        let fname = fname.into();

        trace!("reading({:?}", fname);

        let data = fs::read_to_string(&fname).expect(&format!("file not found {:?}", fname));

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

        trace!("load sources");
        // Register sources
        //
        let src = Sources::load(&None);
        let src = match src {
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
        let basedir: PathBuf = cfg.basedir.unwrap_or(PathBuf::from("/var/run/acute"));
        let pidfile: PathBuf = makepath!(&basedir, ENGINE_PID);
        fs::write(&pidfile, format!("{pid}")).expect("can not write fetiched.pid");

        info!("PID {} written in {:?}", pid, pidfile);

        Engine {
            pid,
            home: Arc::new(basedir),
            sources: Arc::new(src),
            storage: Arc::new(areas),
        }
    }

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

    /// Create a new job queue
    ///
    pub fn create_job(&self, s: &str) -> Job {
        Job::new(s)
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

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Register the state of the running `Engine`.
///
/// NOTE: At the moment, the is not `fetiched` daemon, it is all in a single
/// binary.
///
#[derive(Clone, Debug)]
pub struct State {}

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
