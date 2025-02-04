use strum::EnumString;

pub mod fetch;
pub mod read;
pub mod stream;

/// Represents different types of data producers that can source data
/// into the processing pipeline.
///
/// Each variant corresponds to a specific data sourcing strategy:
///
#[derive(Debug, EnumString, PartialEq, strum::VariantNames)]
pub enum Producer {
    /// Producer that fetches data from remote sources
    Fetch,
    /// Producer that reads data from local files
    Read,
    /// Producer that streams data from a continuous source
    Stream,
}
