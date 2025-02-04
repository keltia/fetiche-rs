//! Regroup all available task/commands
//!

use std::fmt::Debug;
use std::sync::mpsc::Receiver;

use enum_dispatch::enum_dispatch;
use serde::Deserialize;
use strum::EnumString;
use tokio::task::JoinHandle;

use crate::{Consumer, Middle, Producer};

/// Task I/O characteristics
///
/// The main principle being that a consumer should not be first in a job queue
/// just like a producer one should not be last.
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

/// Task represents different types of tasks that can be performed in the data processing pipeline.
///
/// Each variant corresponds to a different stage in the pipeline:
/// - Producer: Tasks that generate or source data
/// - Middle: Tasks that transform or process data
/// - Consumer: Tasks that consume or store the final data
///
#[enum_dispatch]
#[derive(Clone, Debug)]
pub enum Task {
    /// Producer task that generates or sources data
    Producer,
    /// Middle task that transforms or processes data
    Middle,
    /// Consumer task that consumes or stores the final data
    Consumer,
}

/// Anything that can be `run()` is Runnable.
///
/// See the `fetiche-macros` crate for a proc-macro that implement the `run()`  wrapper for
/// the `Runnable` trait.
///
#[allow(async_fn_in_trait)]
#[enum_dispatch(Task)]
pub trait Runnable: Debug {
    fn cap(&self) -> IO;
    async fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<eyre::Result<()>>);
}
