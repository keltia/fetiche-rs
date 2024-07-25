use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use eyre::Result;
#[cfg(unix)]
use home::home_dir;
use log::debug;
use serde::{Deserialize, Serialize};
use tracing::trace;

use fetiche_common::makepath;

use crate::error::Status;

/// This is the package config tag or category.
const CONFIG_TAG: &str = "drone-utils";
#[cfg(unix)]
const BASEDIR: &str = ".config";
/// Config filename
const CONFIG: &str = "process-data.hcl";
/// Current version
const CVERSION: usize = 2;

/// Configuration for the CLI tool
///
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    /// Version in the file MUST match `CVERSION`
    pub version: usize,
    /// Database name or path.
    pub database: Option<String>,
    /// Directory holding the parquet files for the datalake.
    pub datalake: Option<String>,
    /// URL
    pub url: String,
    /// User to connect with
    pub user: Option<String>,
    /// Corresponding password
    pub password: Option<String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        ConfigFile {
            version: CVERSION,
            database: None,
            datalake: None,
            url: "".to_string(),
            user: None,
            password: None,
        }
    }
}

impl ConfigFile {
    /// Returns the path of the default config directory
    ///
    #[cfg(unix)]
    pub fn config_path() -> PathBuf {
        let homedir = home_dir().unwrap();
        let def: PathBuf = makepath!(homedir, BASEDIR, CONFIG_TAG);
        def
    }

    /// Returns the path of the default config directory
    ///
    #[cfg(windows)]
    pub fn config_path() -> PathBuf {
        let homedir = env!("LOCALAPPDATA");

        let def: PathBuf = makepath!(homedir, CONFIG_TAG);
        def
    }

    /// Returns the path of the default config file
    ///
    pub fn default_file() -> PathBuf {
        Self::config_path().join(CONFIG)
    }

    /// Load either file specified as parameter or the default file.
    ///
    #[tracing::instrument]
    pub fn load(fname: &str) -> Result<ConfigFile>
    {
        trace!("loading config");
        let data = fs::read_to_string(fname)
            .map_err(|_| {
                let fname = Self::default_file().to_string_lossy().to_string();
                Status::MissingConfig(fname)
            })?;
        let data: ConfigFile = hcl::from_str(&data)
            .map_err(|e| Status::MissingConfigParameter(e.to_string()))?;
        debug!("config: {:?}", data);

        if data.version != CVERSION {
            return Err(Status::BadFileVersion(data.version).into());
        }
        Ok(data)
    }
}
