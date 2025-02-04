mod archive;
mod save;
mod store;
mod stdout;

pub use archive::*;
pub use save::*;
pub use stdout::*;
pub use store::*;

use crate::{Runnable, Task, IO};

/// Represents different types of consumers that can process and store data
/// in the processing pipeline.
///
/// Each variant corresponds to a specific data consuming strategy:
///
#[derive(Clone, Debug, PartialEq, strum::EnumString, strum::VariantNames)]
pub enum Consumer {
    /// Consumer that display data on screen
    Stdout(Stdout),
    /// Consumer that saves data to temporary storage
    Save,
    /// Consumer that stores data in permanent storage
    Store,
}
