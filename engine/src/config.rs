//! Engine configuration schema and scheduler defaults.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Duration;

use eyre::Result;
use serde::Deserialize;

use fetiche_common::{IntoConfig, Versioned};
use fetiche_macros::into_configfile;

use crate::{EngineMode, EngineStatus, StorageConfig};

/// Configuration filename
pub(crate) const ENGINE_CONFIG: &str = "engine.hcl";

/// Configuration file version
pub(crate) const ENGINE_VERSION: usize = 3;

/// Clock tick
const TICK: Duration = Duration::from_secs(2);

/// Sync state every 30s by default.
const SYNC: Duration = Duration::from_secs(30);

/// Default number of queued messages per pipeline edge.
pub(crate) const DEFAULT_PIPELINE_QUEUE_CAPACITY: usize = 16;
/// The `EngineConfig` struct provides the configuration options used to initialize
/// and manage the `Engine`. It is loaded from the `engine.hcl` file or other sources
/// and defines the base runtime parameters for the engine.
///
/// # Fields
///
/// - `basedir`
///     The base directory where engine-related files, such as state, jobs, and tasks,
///     are stored. This path acts as the root directory for engine operations.
///
/// - `workers`
///     Initial number of runner spawned by the factory.
///
/// - `storage`
///     A `BTreeMap` defining various storage configurations. This can include
///     settings for in-memory caching, directory-based storage, or Hive-based
///     sharding. Each storage configuration must conform to the `StorageConfig` enum.
///
#[into_configfile(version = 3, filename = "engine.hcl")]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct EngineConfig {
    /// Base directory
    pub basedir: PathBuf,
    /// Number of workers for the runner factory.
    pub workers: Option<usize>,
    /// Number of queued messages per pipeline edge.
    pub pipeline_queue_capacity: Option<usize>,
    /// Maximum file size in bytes for a configured Read stage.
    pub read_max_bytes: Option<NonZeroU32>,
    /// Sync the engine state every 30 seconds by default
    pub sync: Option<Duration>,
    /// This is our clock: we get a message every 2 seconds by default
    pub tick: Option<Duration>,
    /// List of storage types
    pub storage: BTreeMap<String, StorageConfig>,
}

impl EngineConfig {
    /// Validates explicitly configured worker and timer settings, then resolves
    /// the effective settings for the selected mode.
    pub(crate) fn runtime_settings(&self, mode: EngineMode) -> Result<(usize, Duration, Duration)> {
        if self.workers == Some(0) {
            return Err(EngineStatus::ZeroWorkers.into());
        }
        for (name, interval) in [("tick", self.tick), ("sync", self.sync)] {
            if interval == Some(Duration::ZERO) {
                return Err(EngineStatus::ZeroInterval(name).into());
            }
        }
        if mode == EngineMode::Single {
            return Ok((1, SYNC, Duration::from_secs(1)));
        }

        let workers = self.workers.unwrap_or_else(|| {
            std::thread::available_parallelism().map_or(1, |workers| workers.get())
        });
        Ok((
            workers,
            self.sync.unwrap_or(SYNC),
            self.tick.unwrap_or(TICK),
        ))
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;
    use crate::producer;

    /// The checked-in HCL deserializes with the current version and settings.
    #[test]
    fn checked_in_hcl_matches_engine_schema() -> Result<()> {
        let config: EngineConfig = hcl::from_str(include_str!("engine.hcl"))?;
        assert_eq!(config.version(), ENGINE_VERSION);
        assert_eq!(
            config.runtime_settings(EngineMode::Daemon)?,
            (2, SYNC, TICK)
        );
        assert_eq!(
            config.runtime_settings(EngineMode::Single)?,
            (1, SYNC, Duration::from_secs(1))
        );
        assert_eq!(
            config.pipeline_queue_capacity,
            Some(DEFAULT_PIPELINE_QUEUE_CAPACITY)
        );
        assert_eq!(
            config.read_max_bytes,
            Some(producer::DEFAULT_READ_MAX_BYTES)
        );
        assert!(matches!(
            config.storage["hourly"],
            StorageConfig::Directory { .. }
        ));
        assert!(matches!(config.storage["hive"], StorageConfig::Hive { .. }));
        assert!(matches!(
            config.storage["cache"],
            StorageConfig::Cache { .. }
        ));
        Ok(())
    }

    /// Zero worker counts and timer intervals fail before the scheduler starts.
    #[rstest::rstest]
    #[case("workers = 0", "workers")]
    #[case("tick = { secs = 0, nanos = 0 }", "tick")]
    #[case("sync = { secs = 0, nanos = 0 }", "sync")]
    fn rejects_zero_runtime_settings(#[case] field: &str, #[case] name: &str) -> Result<()> {
        let input = format!("version = 3\nbasedir = \".\"\nstorage = {{}}\n{field}");
        let config: EngineConfig = hcl::from_str(&input)?;
        let error = config.runtime_settings(EngineMode::Daemon).unwrap_err();
        let field = match error.downcast_ref::<EngineStatus>() {
            Some(EngineStatus::ZeroWorkers) => "workers",
            Some(EngineStatus::ZeroInterval(field)) => field,
            _ => panic!("unexpected error: {error:?}"),
        };
        assert_eq!(field, name);
        Ok(())
    }
}
