pub use save::*;
pub use store::*;

mod save;
mod store;

/// Represents different types of consumers that can process and store data
/// in the processing pipeline.
///
/// Each variant corresponds to a specific data consuming strategy:
///
#[derive(Clone, Debug, PartialEq, strum::EnumString, strum::VariantNames)]
pub enum Consumer {
    /// Consumer that saves data to temporary storage
    Save,
    /// Consumer that stores data in permanent storage
    Store,
}
