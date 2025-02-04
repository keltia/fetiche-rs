use strum::EnumString;

pub mod convert;
pub mod tee;

/// Represents different types of filters that can be applied to the data
/// in the processing pipeline.
///
/// Each variant corresponds to a specific filtering strategy:
///
#[derive(Clone, Debug, EnumString, PartialEq, strum::VariantNames)]
pub enum Filter {
    /// Filter that transforms data from one format to another
    Convert,
    ///Filter that creates a copy of the data stream while passing through
    Tee,
}
