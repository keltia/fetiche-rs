//!
//!

use std::fmt::{write, Display};

use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tracing::error;

mod archive;
mod save;
mod stdout;
mod store;

pub use archive::*;
pub use save::*;
pub use stdout::*;
pub use store::*;

use crate::{Runnable, Stats, Task, IO};

/// Represents different types of consumers that can process and store data
/// in the processing pipeline.
///
/// Each variant corresponds to a specific data consuming strategy:
///
#[derive(Clone, Debug, Default, PartialEq, strum::EnumString, strum::VariantNames)]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Consumer {
    /// Consumer that takes stored data and archives it
    Archive(Archive),
    /// Consumer that saves data to temporary storage
    Save(Save),
    /// Consumer that display data on screen
    Stdout(Stdout),
    /// Consumer that stores data in permanent storage
    Store(Store),
    /// Invalid consumer
    #[default]
    Invalid,
}

impl Runnable for Consumer {
    fn cap(&self) -> IO {
        IO::Consumer
    }

    async fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<eyre::Result<()>>) {
        match self {
            Consumer::Save(c) => { c.run(out).await }
            Consumer::Store(c) => { c.run(out).await }
            Consumer::Stdout(s) => { s.run(out).await }
            Consumer::Invalid => {
                error!("Invalid consumer: {}", self);
                panic!("Invalid consumer: {}", self);
            }
        }
    }
}

impl Display for Consumer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Consumer::Save(_) => write!(f, "Save"),
            Consumer::Store(_) => write!(f, "Store"),
            Consumer::Stdout(_) => write!(f, "Stdout"),
            Consumer::Invalid => write!(f, "Invalid"),
        }
    }
}
