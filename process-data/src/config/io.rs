use eyre::eyre;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::trace;

use crate::makepath;

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Config filename
const CONFIG: &str = "process-data.hcl";
/// Current version
const CVERSION: usize = 1;

/// Configuration for the CLI tool
///
#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    /// Version in the file MUST match `CVERSION`
    pub version: usize,
    /// Database name or path.
    pub database: Option<String>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        ConfigFile {
            version: CVERSION,
            database: None,
        }
    }
}

impl ConfigFile {
    /// Returns the path of the default config directory
    ///
    #[cfg(unix)]
    pub fn config_path() -> PathBuf {
        let homedir = home_dir().unwrap();
        let def: PathBuf = makepath!(homedir, BASEDIR, "drone-utils");
        def
    }

    /// Returns the path of the default config directory
    ///
    #[cfg(windows)]
    pub fn config_path() -> PathBuf {
        let homedir = env!("LOCALAPPDATA");

        let def: PathBuf = makepath!(homedir, "drone-utils");
        def
    }

    /// Returns the path of the default config file
    ///
    pub fn default_file() -> PathBuf {
        Self::config_path().join(CONFIG)
    }

    /// Load either file specified as parameter or the default file if `None`.
    ///
    #[tracing::instrument]
    pub fn load(fname: Option<PathBuf>) -> eyre::Result<ConfigFile> {
        trace!("loading config");
        let fname = match fname {
            Some(fname) => fname,
            _ => Self::default_file(),
        };

        let data = fs::read_to_string(fname)?;
        let data: ConfigFile = hcl::from_str(&data)?;

        if data.version != CVERSION {
            return Err(eyre!("bad file version: {}", data.version));
        }
        Ok(data)
    }
}
