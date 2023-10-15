use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use eyre::{eyre, Result};
#[cfg(unix)]
use home::home_dir;
use serde::Deserialize;
use tracing::trace;

use fetiche_sources::{makepath, Auth};

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Config filename
const CONFIG: &str = "config.hcl";
/// Current version
const CVERSION: usize = 1;

/// Configuration for the CLI tool, supposed to include parameters and most importantly
/// credentials for the various sources.
///
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Version in the file MUST match `CVERSION`
    pub version: usize,
    /// Each site credentials
    pub site: BTreeMap<String, Auth>,
}

impl Config {
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

    #[tracing::instrument]
    pub fn load(fname: Option<PathBuf>) -> Result<Config> {
        trace!("loading config");
        let fname = match fname {
            Some(fname) => fname,
            _ => Self::default_file(),
        };

        let data = fs::read_to_string(fname)?;
        let data: Config = hcl::from_str(&data)?;

        if data.version != CVERSION {
            return Err(eyre!("bad file version: {}", data.version));
        }
        Ok(data)
    }
}
