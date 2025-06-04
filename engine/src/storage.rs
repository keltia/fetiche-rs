use std::collections::BTreeMap;
use std::num::ParseIntError;
use std::path::PathBuf;

use crate::StorageError;
use eyre::Result;
use nom::{
    character::complete::{i8, one_of}, combinator::map_res,
    IResult,
    Parser,
};
use serde::Deserialize;
use strum::EnumString;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::{debug, trace};

/// The `StorageConfig` enum defines different storage types supported by the engine.
/// It allows the engine to specify and configure storage modules based on the operational
/// requirements (e.g., in-memory caching, local filesystem storage, or Hive-based sharding).
///
/// # Variants
///
/// - `Cache`
///     Defines an in-memory key-value store configuration, typically connected to a service
///     like DragonflyDB or REDIS. Requires a `url` to connect.
///
/// - `Directory`
///     Represents storage based on the local filesystem. Includes a `path` to the directory
///     and a `rotation` mechanism for maintaining storage consistency or archival.
///
/// - `Hive`
///     Adds support for Hive-based sharding. Designed for scalable and distributed storage.
///     Includes a `path` for file-based Hive shards.
///
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum StorageConfig {
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache { url: String },
    /// In the local filesystem
    Directory { path: PathBuf, rotation: String },
    /// HIVE-based sharding
    Hive { path: PathBuf },
}

/// This is the part describing the available storage areas
///
#[derive(Clone, Debug)]
pub struct Storage(BTreeMap<String, StoreArea>);

/// `StoreArea` represents the different types of storage locations that can
/// be used in the application. These enums describe and configure a specific
/// storage type, such as caching, local directories, or HIVE-based sharding.
///
/// Variants:
/// - `Cache`: Represents an in-memory key/value cache (e.g., REDIS).
///   - `url`: The connection URL for the cache.
/// - `Directory`: Represents a storage location on the local filesystem.
///   - `path`: Specifies the path to the directory on the system.
///   - `rotation`: The time-based rotation configuration, defined in seconds.
/// - `Hive`: Represents HIVE-based sharding storage.
///   - `path`: Specifies the path to the Hive sharded storage system.
///
/// The `serialize_all = "PascalCase"` attribute ensures the enum variants
/// are serialized in PascalCase (e.g., `Cache`, `Directory`, `Hive`).
///
/// Example usage:
/// ```rust
/// use std::path::PathBuf;
/// use fetiche_engine::StoreArea;
///
/// let cache = StoreArea::Cache { url: "redis://127.0.0.1:6379".to_string() };
/// let directory = StoreArea::Directory {
///     path: PathBuf::from("/tmp/data"),
///     rotation: 3600
/// };
/// let hive = StoreArea::Hive { path: PathBuf::from("/sharded/storage") };
/// ```
///
#[derive(Clone, Debug, EnumString, strum::Display)]
#[strum(serialize_all = "PascalCase")]
pub enum StoreArea {
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache { url: String },
    /// In the local filesystem
    Directory { path: PathBuf, rotation: u32 },
    /// HIVE-based sharding
    Hive { path: PathBuf },
}

impl Storage {
    ///
    /// This method registers all storage areas from a configuration struct (`StorageConfig`)
    /// that is read from an `engine.hcl` file.
    ///
    /// The function iterates through the configuration provided as a BTreeMap, identifying
    /// and creating the corresponding storage areas while enforcing necessary configurations
    /// like validating or creating directories and parsing rotation values.
    ///
    /// The configurations are then stored in a `BTreeMap` inside the `Storage` struct, with
    /// each entry corresponding to a specific storage area name and its details.
    ///
    /// ### Parameters:
    /// - `cfg`: A reference to a `BTreeMap<String, StorageConfig>` that contains the configurations
    ///          for various storage areas.
    ///
    /// ### Returns:
    /// - `Storage`: A `Storage` struct containing the registered storage areas.
    ///
    /// ### Panics:
    /// This method panics:
    /// - If it fails to create directories specified in `StorageConfig::Directory` or
    ///   `StorageConfig::Hive` entries.
    ///
    /// ### Example:
    /// ```rust
    /// use std::collections::BTreeMap;
    /// use fetiche_engine::{Storage, StorageConfig};
    ///
    /// let mut config = BTreeMap::new();
    /// config.insert(
    ///     "local".to_string(),
    ///     StorageConfig::Directory {
    ///         path: std::path::PathBuf::from("/tmp/data"),
    ///         rotation: "1h".to_string(),
    ///     },
    /// );
    ///
    /// let storage = Storage::register(&config);
    /// ```
    ///
    #[tracing::instrument]
    pub fn register(cfg: &BTreeMap<String, StorageConfig>) -> Result<Self> {
        trace!("load storage areas");

        let mut b = BTreeMap::<String, StoreArea>::new();

        for (name, area) in cfg.into_iter() {
            match area {
                // Local directory
                //
                StorageConfig::Directory { path, rotation } => {
                    if !path.exists() {
                        if let Err(_) = std::fs::create_dir_all(path) {
                            let path = path.to_string_lossy().to_string();
                            return Err(StorageError::CannotCreateTree(path).into());
                        }
                    }
                    let rotation = match Self::parse_rotation(rotation.as_str()) {
                        Ok((_, v)) => v,
                        Err(_e) => {
                            return Err(eyre::eyre!(
                                "Invalid rotation value for storage '{name}': {rotation}"
                            ));
                        }
                    };
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
                // Future HIVE support
                //
                StorageConfig::Hive { path } => {
                    if !path.exists() {
                        if let Err(_) = std::fs::create_dir_all(path) {
                            let path = path.to_string_lossy().to_string();
                            return Err(StorageError::CannotCreateTree(path).into());
                        }
                    }
                    b.insert(name.to_string(), StoreArea::Hive { path: path.clone() });
                }
            }
        }
        debug!("b={:?}", b);
        Ok(Storage(b))
    }

    ///
    /// List all registered storage areas in a formatted table.
    ///
    /// This method iterates over all entries in the internal `BTreeMap` of the `Storage` struct
    /// and generates a human-readable table containing each storage area's name, path/URL, and
    /// (if applicable) its rotation interval.
    ///
    /// ### Returns:
    /// - `Ok(String)`: A string containing the formatted table of storage areas.
    /// - `Err`: If any error occurs during the construction of the table.
    ///
    /// ### Example:
    /// ```rust
    /// use std::collections::BTreeMap;
    /// use fetiche_engine::{Storage, StorageConfig};
    ///
    /// let mut config = BTreeMap::new();
    /// config.insert(
    ///     "cache".to_string(),
    ///     StorageConfig::Cache {
    ///         url: "redis://localhost:6379".to_string(),
    ///     },
    /// );
    /// config.insert(
    ///     "local".to_string(),
    ///     StorageConfig::Directory {
    ///         path: std::path::PathBuf::from("/tmp/data"),
    ///         rotation: "5h".to_string(),
    ///     },
    /// );
    ///
    /// let storage = Storage::register(&config);
    /// println!("{}", storage.list().unwrap());
    /// ```
    ///
    /// ### Panics:
    /// This method does not explicitly panic but may propagate panics from underlying operations
    /// (e.g., string formatting or table construction errors).
    ///
    pub fn list(&self) -> Result<String> {
        let header = vec!["Name", "Path/URL", "Rotation"];

        let mut builder = Builder::default();
        builder.push_record(header);

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
                StoreArea::Hive { path } => {
                    let path = path.to_string_lossy();
                    row.push(path.to_string());
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

    ///
    /// Parse the rotation interval from a string input.
    ///
    /// This method accepts a string representation of time intervals using suffixes indicating
    /// the unit of time, such as "s" for seconds, "m" for minutes, "h" for hours, and "d" for days.
    ///
    /// ### Supported Formats:
    /// - `42s`: 42 seconds
    /// - `2m`: 2 minutes (converted to 120 seconds)
    /// - `1h`: 1 hour (converted to 3600 seconds)
    /// - `1d`: 1 day (converted to 86400 seconds)
    ///
    /// ### Arguments:
    /// - `input`: A string slice that represents the time interval (e.g., `"5h"`).
    ///
    /// ### Returns:
    /// - `Ok((&str, u32))`: The remaining input and the resulting interval in seconds.
    /// - `Err`: In case of invalid input or parsing error.
    ///
    /// ### Example:
    /// ```rust
    /// use nom::IResult;
    /// use fetiche_engine::Storage;
    ///
    /// let result: IResult<&str, u32> = Storage::parse_rotation("5h");
    /// assert_eq!(result, Ok(("", 18000)));
    ///
    /// let result: IResult<&str, u32> = Storage::parse_rotation("1d");
    /// assert_eq!(result, Ok(("", 86400)));
    /// ```
    ///
    /// ### Notes:
    /// - The method utilizes the [`nom`](https://docs.rs/nom/) library for parsing.
    /// - The parsing is case-sensitive and expects valid formats as described above.
    ///
    pub fn parse_rotation(input: &str) -> IResult<&str, u32> {
        let into_seconds =
            |(n, tag): (std::primitive::i8, char)| -> std::result::Result<u32, ParseIntError> {
                let res = match tag {
                    's' => n as u32,
                    'm' => (n as u32) * 60,
                    'h' => (n as u32) * 3_600,
                    'd' => (n as u32) * 3_600 * 24,
                    _ => n as u32,
                };
                Ok(res)
            };

        map_res((i8, one_of("smhd")), into_seconds).parse(input)
    }

    pub fn insert<T: Into<String>>(&mut self, key: T, val: StoreArea) -> Option<StoreArea> {
        self.0.insert(key.into(), val)
    }
}

#[cfg(test)]
mod tests {
    use jiff::{Span, SpanRelativeTo, Unit};
    use rstest::rstest;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    use crate::{Storage, StorageConfig};

    #[rstest]
    #[case("42s", 42_u32)]
    #[case("60s", 60_u32)]
    #[case("2m", 120_u32)]
    #[case("5h", 18_000_u32)]
    #[case("24h", 86_400_u32)]
    #[case("1d", 86_400_u32)]
    fn test_parse_rotation(#[case] input: &str, #[case] val: u32) {
        let (_, v) = Storage::parse_rotation(input).unwrap();
        assert_eq!(val, v);
    }

    #[rstest]
    #[case("42s", 42_f64)]
    #[case("60s", 60_f64)]
    #[case("2m", 120_f64)]
    #[case("5h", 18_000_f64)]
    #[case("24h", 86_400_f64)]
    #[case("1d", 86_400_f64)]
    fn test_parse_rotation_jiff(#[case] input: &str, #[case] val: f64) -> eyre::Result<()> {
        let marker = SpanRelativeTo::days_are_24_hours();
        let v: Span = input.parse()?;
        assert_eq!(v.total((Unit::Second, marker))?, val);
        Ok(())
    }

    #[test]
    fn test_register_cache() {
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "test_cache".to_string(),
            StorageConfig::Cache {
                url: "redis://localhost:6379".to_string(),
            },
        );

        let storage = Storage::register(&cfg).unwrap();
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_register_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "test_dir".to_string(),
            StorageConfig::Directory {
                path: temp_dir.path().to_path_buf(),
                rotation: "1h".to_string(),
            },
        );

        let storage = Storage::register(&cfg).unwrap();
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_register_hive() {
        let temp_dir = TempDir::new().unwrap();
        let mut cfg = BTreeMap::new();
        cfg.insert(
            "test_hive".to_string(),
            StorageConfig::Hive {
                path: temp_dir.path().to_path_buf(),
            },
        );

        let storage = Storage::register(&cfg).unwrap();
        assert_eq!(storage.len(), 1);
    }
}
