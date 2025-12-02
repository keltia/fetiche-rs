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
use std::str::FromStr;

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

impl From<String> for Frame {
    fn from(s: String) -> Self {
        Frame::Bytes(s.into_bytes().into())
    }
}

impl FromStr for Frame {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Frame::Bytes(s.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_from_string() {
        let s = String::from("test");
        let frame = Frame::from(s);
        match frame {
            Frame::Bytes(b) => assert_eq!(b, Bytes::from("test")),
            _ => panic!("Expected Frame::Bytes"),
        }
    }

    #[test]
    fn test_frame_from_str() {
        let s = "test";
        let frame = Frame::from_str(s).unwrap();
        match frame {
            Frame::Bytes(b) => assert_eq!(b, Bytes::from("test")),
            _ => panic!("Expected Frame::Bytes"),
        }
    }

    #[test]
    fn test_frame_default() {
        let frame = Frame::default();
        assert!(matches!(frame, Frame::Null));
    }
}

