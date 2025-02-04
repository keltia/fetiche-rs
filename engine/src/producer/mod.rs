use fetiche_macros::RunnableDerive;
use std::fmt::Display;
use strum::EnumString;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tracing::error;

pub use dummy::*;
pub use fetch::*;
pub use read::*;
pub use stream::*;

use crate::{Consumer, Runnable, Sources, Stats, Task, IO};

mod dummy;
mod fetch;
mod read;
mod stream;

/// Represents different types of data producers that can source data
/// into the processing pipeline.
///
/// Each variant corresponds to a specific data sourcing strategy:
///
#[derive(Clone, Debug, Default, EnumString, PartialEq, strum::VariantNames)]
pub enum Producer {
    /// Dummy `Producer` for tests
    Dummy(Dummy),
    /// Producer that fetches data from remote sources
    Fetch(Fetch),
    /// Producer that reads data from local files
    Read(Read),
    /// Producer that streams data from a continuous source
    Stream(Stream),
    /// Invalid
    #[default]
    Invalid,
}

impl From<Producer> for Task {
    fn from(value: Producer) -> Self {
        Task::Producer(value)
    }
}

impl Runnable for Producer {
    fn cap(&self) -> IO {
        IO::Producer
    }

    #[tracing::instrument(skip(self))]
    async fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<eyre::Result<()>>) {
        match self {
            Producer::Dummy(dummy) => dummy.run(out).await,
            Producer::Fetch(p) => p.run(out).await,
            Producer::Read(p) => p.run(out).await,
            Producer::Stream(p) => p.run(out).await,
            Producer::Invalid => {
                error!("Invalid producer: {}", self);
                panic!(
                    "Invalid producer: {}",
                    self
                )
            }
        }
    }
}

impl Display for Producer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Producer::Dummy(_) => write!(f, "Dummy"),
            Producer::Fetch(_) => write!(f, "Fetch"),
            Producer::Read(_) => write!(f, "Read"),
            Producer::Stream(_) => write!(f, "Stream"),
            Producer::Invalid => write!(f, "Invalid"),
        }
    }
}
