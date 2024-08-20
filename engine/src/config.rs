use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

use fetiche_common::{IntoConfig, Versioned};
use fetiche_macros::into_configfile;

/// Configuration file format
#[into_configfile(version = 2, filename = "engine.hcl")]
#[derive(Clone, Debug, Deserialize)]
pub struct EngineConfig {
    /// Base directory
    pub basedir: PathBuf,
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
    /// HIVE-based sharding
    Hive { path: PathBuf },
}
