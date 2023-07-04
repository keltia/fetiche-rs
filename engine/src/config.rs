use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

/// Configuration file format
///
#[derive(Clone, Debug, Deserialize)]
pub struct EngineConfig {
    /// Usual check for malformed file
    pub version: usize,
    /// List of storage types
    pub storage: BTreeMap<String, StorageConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum StorageConfig {
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache { url: String },
    /// In the local filesystem
    Directory { path: PathBuf, rotation: String },
}
