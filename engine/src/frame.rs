//! Frame represents the core data structure used for passing information between producers and consumers
//! in the Fetiche framework. It is implemented as an enum to avoid generic type parameters.
//!
//! # Features
//!
//! The Frame enum supports different data types based on enabled feature flags:
//!
//! - `asd`: Support for ASD (Aircraft Situation Display) data format
//! - `avionix`: Support for Avionix cube data format
//! - `senhive`: Support for Senhive fused data format
//!
//! # Generic Frames
//!
//! - `Bytes`: Raw byte data
//! - `Error`: Error messages
//! - `Null`: Empty/default frame
//!

use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[cfg(feature = "avionix")]
use fetiche_formats::avionix::CubeData;
#[cfg(feature = "senhive")]
use fetiche_formats::senhive::FusedData;
#[cfg(feature = "asd")]
use fetiche_formats::Asd;

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum Frame {
    // Generic frames
    Bytes(Bytes),
    Error(String),
    #[default]
    Null,
    // Source-specific frames
    #[cfg(feature = "asd")]
    Asd(Vec<Asd>),
    #[cfg(feature = "avionix")]
    Avionix(Vec<CubeData>),
    #[cfg(feature = "senhive")]
    Senhive(Vec<FusedData>),
}

