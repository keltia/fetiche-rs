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
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;

use anyhow::Result;

use fetiche_formats::Format;
use fetiche_sources::{makepath, Fetchable, Sources, Streamable};
pub use job::*;
pub use task::*;

use crate::StoreArea::Directory;

mod job;
mod parse;
mod task;

pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Configuration filename
const ENGINE_CONFIG: &str = "engine.hcl";

/// Configuration file version
const ENGINE_VERSION: usize = 1;

/// We define a `Store` enum, describing storage areas like a directory or an S3
/// bucket (from an actual AWS account or a Garage instance).
///
#[derive(Clone, Debug)]
pub enum StoreArea {
    /// S3 AWS/Garage bucket
    Bucket { name: String },
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache,
    /// In the local filesystem
    Directory { path: PathBuf },
}

pub struct StorageAreas(BTreeMap<String, StoreArea>);

/// Main `Engine` struct that hold the sources and everything needed to perform
///
#[derive(Clone, Debug)]
pub struct Engine {
    /// Sources
    pub sources: Arc<Sources>,
    /// Storage area for long running jobs
    pub storage: Arc<StoreAreas>,
}

impl Engine {
    pub fn new() -> Self {
        // Load storage areas
        //
        let fname = Self::default_file();
        let areas = match fs::read_to_string(fname) {
            Ok(data) => {
                let store: BTreeMap<String, StoreArea> = hcl::from_str(&data).unwrap();
                store
            }
            Err(e) => panic!("No storage define in {}:{}", fname.to_string_lossy(), e),
        };

        // Register sources
        //
        let src = Sources::load(&None);
        let src = match src {
            Ok(src) => src,
            Err(e) => panic!("No sources configured in 'sources.hcl':{}", e),
        };

        Engine {
            sources: Arc::new(src),
            storage: Arc::new(areas),
        }
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

    // Load configuration file for storage areas
    //
    pub fn with(fname: &str) -> Self {
        let cfg = fs::read_to_string(fname);
    }

    /// Initialize the optional storage area for jobs' output files
    ///
    pub fn store(&mut self, path: &str) -> &mut Self {
        let path = PathBuf::from(path);
        if !path.exists() {
            create_dir_all(&path).expect("create_dir_all failed");
        }
        self.storage = Some(Directory { path });
        self
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

    /// Create a new job queue
    ///
    pub fn create_job(&self, s: &str) -> Job {
        Job::new(s, Arc::clone(&self.sources))
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of task we will need to do
///
#[derive(Clone, Debug, Default)]
pub enum Input {
    /// File-based means we need the formats beforehand and a pathname
    ///
    File {
        /// Input formats
        format: Format,
        /// Path of the input file
        path: PathBuf,
    },
    /// Network-based means we need the site name (whose details are taken from the configuration
    /// file.  The `site` is a `Fetchable` object generated from `Config`.
    ///
    Network {
        /// Input formats
        format: Format,
        /// Site itself
        site: Arc<dyn Fetchable>,
    },
    /// Stream-based means we need the site name (whose details are taken from the configuration
    /// file.  The `site` is a `Streamable` object generated from `Config`.
    ///
    Stream {
        /// Input formats
        stream: Format,
        /// Site itself
        site: Arc<dyn Streamable>,
    },
    #[default]
    Nothing,
}

/// Task I/O characteristics
///
/// The main principle being that a consumer should not be first in a job queue
/// just like an Out one should not be last.
///
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
/// See the engine-macro crate for a rpoc-macro that implement the `run()`  wrapper for
/// the `Runnable` trait.
///
pub trait Runnable: Debug {
    fn cap(&self) -> IO;
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}
