//! Library implementing common part of the transformations
//!
//! In the Engine, we run jobs.  Jobs are made from a list a Task and all tasks are put into
//! a pipeline.  All tasks must be Runnable and the RunnableDerive macro stitches everything
//! together with channels.
//!
//! Most jobs will be fetch or stream with a conversion task at the end, etc.
//! For the first task, the stdin channel will just serve as a trigger for the pipeline.
//!
//! Each Runnable task will be marked as RunnableDerive and will need to define a transform()
//! member function for the main task.  It takes the previous stage output as a string and should
//! return a string with the transformed output that will be sent to the next stage.
//!

use std::convert::Into;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use anyhow::{anyhow, Result};

pub use common::*;
pub use fetch::*;
use fetiche_formats::Format;
use fetiche_sources::{Fetchable, Sources, Streamable};
pub use into::*;
pub use job::*;
pub use parse::*;
pub use read::*;
pub use stream::*;

mod common;
mod fetch;
mod into;
mod job;
mod parse;
mod read;
mod stream;

pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// Main `Engine` struct that hold the sources and everything needed to perform
///
#[derive(Clone, Debug)]
pub struct Engine {
    /// Sources
    pub sources: Arc<Sources>,
}

impl Engine {
    pub fn new() -> Self {
        let src = Sources::load(&None);
        match src {
            Ok(src) => Engine {
                sources: Arc::new(src),
            },
            _ => panic!("No sources configured"),
        }
    }

    pub fn from(fname: &str) -> Self {
        let src = Sources::load(&Some(fname.into()));
        match src {
            Ok(src) => Engine {
                sources: Arc::new(src),
            },
            _ => panic!("No sources configured in {fname}"),
        }
    }

    /// Return an `Arc::clone` of the Engine sources
    ///
    pub fn sources(&self) -> Arc<Sources> {
        Arc::clone(&self.sources)
    }

    /// Return a description of all supported sources
    ///
    pub fn list_sources(&self) -> Result<String> {
        Ok(self.sources.list()?)
    }

    /// Return a descriptions of all supported data formats
    ///
    pub fn list_formats(&self) -> Result<String> {
        Ok(Format::list()?)
    }

    /// Return a list of all currently available authentication tokens
    ///
    pub fn list_tokens(&self) -> Result<String> {
        Ok(self.sources.list_tokens()?)
    }
}

enum Task {
    Copy,
    Fetch,
    Message,
    Read,
    Stream,
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
    Stream {
        /// Input formats
        stream: Format,
        /// Site itself
        site: Arc<dyn Streamable>,
    },
    #[default]
    Nothing,
}

/// Anything that can be `run()` is runnable.
///
/// See the engine-macro crate for a rpoc-macro that implement the `run()`  wrapper for
/// the `Runnable` trait.
///
pub trait Runnable: Debug {
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}
