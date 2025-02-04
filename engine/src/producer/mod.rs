pub use fetch::*;
use fetiche_macros::RunnableDerive;
pub use read::*;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
pub use stream::*;
use strum::EnumString;
use tracing::error;

use crate::{Runnable, Site, Task, IO};

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
    /// Producer that fetches data from remote sources
    Fetch,
    /// Producer that reads data from local files
    Read,
    /// Producer that streams data from a continuous source
    Stream,
}
