use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
#[cfg(unix)]
use home::home_dir;
use serde::Deserialize;

use fetiche_sources::{makepath, Auth};

#[cfg(unix)]
const BASEDIR: &str = ".config";

const CONFIG: &str = "config.hcl";
const CVERSION: usize = 1;

/// Configuration for the CLI tool, supposed to include parameters and most importantly
/// credentials for the various sources.
///
#[derive(Debug, Deserialize)]
pub struct Config {
    pub version: usize,
    pub site: BTreeMap<String, Entry>,
}

/// Hold credentials
///
#[derive(Debug, Deserialize)]
pub struct Entry {
    auth: Auth,
}

impl Default for Entry {
    fn default() -> Self {
        Entry { auth: Auth::Anon }
    }
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

    pub fn load(fname: Option<PathBuf>) -> Result<Config> {
        let fname = match fname {
            Some(fname) => fname,
            _ => Self::default_file(),
        };
        let data = fs::read_to_string(fname).expect("Can not open config.hcl");

        let data: Config = hcl::from_str(&data)?;
        if data.version != CVERSION {
            return Err(anyhow!("bad file version: {}", data.version));
        }
        Ok(data)
    }
}
