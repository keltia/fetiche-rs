//! Define what we consider a "container", that is, a file format.
//!
//! This is different from a "data" format which is why it is here.
//!
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};

/// This struct holds the different container formats that we support.
///
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    strum::Display,
    EnumString,
    Serialize,
    VariantNames,
)]
#[strum(serialize_all = "PascalCase", ascii_case_insensitive)]
pub enum Container {
    /// Apache Avro
    CSV,
    /// Apache Parquet
    Parquet,
    /// RAW Files
    #[default]
    Raw,
}
