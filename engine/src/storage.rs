use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;

use crate::StorageConfig;

/// This is the part describing the available storage areas
///
#[derive(Clone, Debug)]
pub struct Storage(BTreeMap<String, StoreArea>);

impl Storage {
    pub fn list(&self) -> Result<String> {
        Ok("".to_owned())
    }
}

/// We define a `Store` enum, describing storage areas like a directory or an S3
/// bucket (from an actual AWS account or a Garage instance).
///
/// FIXME: S3 support require async which we will not do yet
///
#[derive(Clone, Debug)]
pub enum StoreArea {
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache { url: String },
    /// In the local filesystem
    Directory { path: PathBuf },
}

impl Storage {
    /// Register all areas from a config struct read from `engine.hcl`
    ///
    pub fn register(cfg: &BTreeMap<String, StorageConfig>) -> Self {
        let mut b = BTreeMap::<String, StoreArea>::new();

        while let Some(name) = cfg.keys().next() {
            let area = cfg.get(name).unwrap().clone();
            match area {
                // Local directory
                //
                StorageConfig::Directory { path } => {
                    if !path.exists() {
                        std::fs::create_dir_all(&path)
                            .expect(&format!("storage::init::create_dir_all failed: {:?}", path));
                    }
                    b.insert(name.to_string(), StoreArea::Directory { path });
                }
                // Future cache support
                //
                StorageConfig::Cache { url } => {
                    b.insert(name.to_string(), StoreArea::Cache { url });
                }
            }
        }
        Storage(b)
    }

    pub fn insert<T: Into<String>>(&mut self, key: T, val: StoreArea) -> Option<StoreArea> {
        self.0.insert(key.into(), val)
    }
}
