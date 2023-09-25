use std::collections::BTreeMap;
use std::path::PathBuf;

use eyre::Result;
use tracing::{debug, trace};

use nom::{
    character::complete::{i8, one_of},
    combinator::map,
    sequence::tuple,
    IResult,
};
use strum::EnumString;
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::StorageConfig;

/// This is the part describing the available storage areas
///
#[derive(Clone, Debug)]
pub struct Storage(BTreeMap<String, StoreArea>);

/// We define a `Store` enum, describing storage areas like a directory or an S3
/// bucket (from an actual AWS account or a Garage instance).
///
/// FIXME: S3 support require async which we will not do yet
///
#[derive(Clone, Debug, EnumString, strum::Display)]
#[strum(serialize_all = "PascalCase")]
pub enum StoreArea {
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache { url: String },
    /// In the local filesystem
    Directory { path: PathBuf, rotation: u32 },
}

impl Storage {
    /// Register all areas from a config struct read from `engine.hcl`
    ///
    #[tracing::instrument]
    pub fn register(cfg: &BTreeMap<String, StorageConfig>) -> Self {
        trace!("load storage areas");

        let mut b = BTreeMap::<String, StoreArea>::new();

        for (name, area) in cfg.iter() {
            match area {
                // Local directory
                //
                StorageConfig::Directory { path, rotation } => {
                    if !path.exists() {
                        std::fs::create_dir_all(path).unwrap_or_else(|_| {
                            panic!("storage::init::create_dir_all failed: {:?}", path)
                        });
                    }
                    let (_, rotation) = Self::parse_rotation(rotation).unwrap();
                    b.insert(
                        name.to_string(),
                        StoreArea::Directory {
                            path: path.clone(),
                            rotation,
                        },
                    );
                }
                // Future cache support
                //
                StorageConfig::Cache { url } => {
                    b.insert(name.to_string(), StoreArea::Cache { url: url.clone() });
                }
            }
        }
        debug!("b={:?}", b);
        Storage(b)
    }

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
                StoreArea::Cache { url } => row.push(url),
                StoreArea::Directory { path, rotation } => {
                    let path = path.to_string_lossy();
                    row.push(path.to_string());
                    row.push(format!("{}s", rotation));
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

    /// Parse 1s/1m/1h/1d
    ///
    fn parse_rotation(input: &str) -> IResult<&str, u32> {
        let into_s = |(n, tag): (std::primitive::i8, char)| match tag {
            's' => n as u32,
            'm' => (n as u32) * 60,
            'h' => (n as u32) * 3_600,
            'd' => (n as u32) * 3_600 * 24,
            _ => n as u32,
        };
        let r = tuple((i8, one_of("smhd")));
        map(r, into_s)(input)
    }

    pub fn insert<T: Into<String>>(&mut self, key: T, val: StoreArea) -> Option<StoreArea> {
        self.0.insert(key.into(), val)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    #[rstest]
    #[case("42s", 42_u32)]
    #[case("60s", 60_u32)]
    #[case("2m", 120_u32)]
    #[case("5h", 18_000_u32)]
    #[case("1d", 86_400_u32)]
    fn test_parse_rotation(#[case] input: &str, #[case] val: u32) {
        let (_, v) = Storage::parse_rotation(input).unwrap();
        assert_eq!(val, v);
    }
}
