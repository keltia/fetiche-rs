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

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use anyhow::Result;

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
#[derive(Debug)]
pub struct Engine {
    /// Sources
    pub sources: Sources,
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
pub trait Runnable: Debug {
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}
