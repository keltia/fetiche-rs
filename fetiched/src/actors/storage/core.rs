//! Main file for the StorageActor data struct & fn
//!

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

use eyre::Result;
use nom::character::complete::u8;
use nom::{character::complete::one_of, combinator::map, sequence::tuple, IResult};
use serde::Deserialize;
use strum::EnumString;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::{debug, trace};

/// This is the part describing the available storage areas
///
#[derive(Clone, Debug)]
pub struct Storage(BTreeMap<String, StorageArea>);

/// We define a `Store` enum, describing storage areas like a directory or an S3
/// bucket (from an actual AWS account or a Garage instance).
///
#[derive(Clone, Debug, Deserialize, EnumString, strum::Display)]
#[serde(untagged)]
#[strum(serialize_all = "PascalCase")]
pub enum StorageArea {
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache { url: String },
    /// In the local filesystem
    Directory { path: PathBuf, rotation: Threshold },
    /// S3 endpoint
    S3 { addr: String, bucket: String },
}

#[derive(Clone, Debug, Deserialize, strum::Display)]
#[serde(untagged)]
pub enum Threshold {
    /// Actual time in seconds
    U32(u32),
    /// String that can be NNNNN (seconds) or something like 32m, 3h or 2d
    Str(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseRotationError;

impl FromStr for Threshold {
    type Err = ParseRotationError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (_, v) = parse_rotation(s).unwrap();
        Ok(Threshold::U32(v))
    }
}

impl Default for Threshold {
    fn default() -> Self {
        Threshold::U32(0)
    }
}

impl Storage {
    /// Register all areas from a config struct read from `engine.hcl`
    ///
    #[tracing::instrument]
    pub fn register(cfg: &BTreeMap<String, StorageArea>) -> Self {
        trace!("load storage areas");

        let mut b = BTreeMap::<String, StorageArea>::new();

        for (name, area) in cfg.iter() {
            match area {
                // Local directory
                //
                StorageArea::Directory { path, rotation } => {
                    if !path.exists() {
                        std::fs::create_dir_all(path).unwrap_or_else(|_| {
                            panic!("storage::init::create_dir_all failed: {:?}", path)
                        });
                    }
                    b.insert(
                        name.to_string(),
                        StorageArea::Directory {
                            path: path.clone(),
                            rotation: rotation.clone(),
                        },
                    );
                }
                // Future cache support
                //
                StorageArea::Cache { url } => {
                    b.insert(name.to_string(), StorageArea::Cache { url: url.clone() });
                }
                // S3 support
                //
                StorageArea::S3 { addr, bucket } => {
                    unimplemented!()
                }
            }
        }
        debug!("b={:?}", b);
        Storage(b)
    }

    /// Returns a nice table with all options
    ///
    pub fn list(&self) -> Result<String> {
        let header = vec!["Name", "Path/URL", "Rotation"];

        let mut builder = Builder::default();
        builder.set_header(header);

        self.0.iter().for_each(|(n, s)| {
            let mut row = vec![];
            let name = n.clone();
            let area = s.clone();
            row.push(name);
            match area {
                StorageArea::Cache { url } => row.push(url),
                StorageArea::Directory { path, rotation } => {
                    let path = path.to_string_lossy();
                    row.push(path.to_string());
                    row.push(format!("{}s", rotation));
                }
                StorageArea::S3 { addr, bucket } => {
                    unimplemented!()
                }
            };
            builder.push_record(row);
        });
        let allc = builder.build().with(Style::modern()).to_string();
        let str = format!("List all storage areas:\n{allc}");
        Ok(str)
    }

    /// Return the number of storage areas
    ///
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check whether it is empty or not
    ///
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert<T: Into<String>>(&mut self, key: T, val: StorageArea) -> Option<StorageArea> {
        self.0.insert(key.into(), val)
    }
}

/// Parse 1s/1m/1h/1d and return the time in seconds
///
fn parse_rotation(input: &str) -> IResult<&str, u32> {
    let into_s = |(n, tag): (std::primitive::u8, char)| match tag {
        's' => n as u32,
        'm' => (n as u32) * 60,
        'h' => (n as u32) * 3_600,
        'd' => (n as u32) * 3_600 * 24,
        _ => n as u32,
    };
    let r = tuple((u8, one_of("smhd")));
    map(r, into_s)(input)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("42s", 42_u32)]
    #[case("60s", 60_u32)]
    #[case("2m", 120_u32)]
    #[case("5h", 18_000_u32)]
    #[case("1d", 86_400_u32)]
    fn test_parse_rotation(#[case] input: &str, #[case] val: u32) {
        let (_, v) = parse_rotation(input).unwrap();
        assert_eq!(val, v);
    }
}
