use strum::EnumString;

pub use common::*;
pub use convert::*;
pub use tee::*;

mod common;
mod convert;
mod tee;

/// Represents different types of filters that can be applied to the data
/// in the processing pipeline.
///
/// Each variant corresponds to a specific filtering strategy that processes
/// or transforms the data as it flows through the pipeline. Filters can modify,
/// duplicate, or pass through data without modification depending on their type.
///
#[derive(Clone, Debug, EnumString, PartialEq, strum::VariantNames)]
pub enum Filter {
    /// Filter that transforms data from one format to another
    Convert,
    /// Filter that creates an identical copy of the incoming data
    Copy,
    /// Filter that processes or transforms messages in the data stream
    Message,
    /// Filter that passes data through without any modification
    Nothing,
    /// Filter that creates a copy of the data stream while passing through
    Tee,
}
