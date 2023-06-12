use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use home::home_dir;
use serde::Deserialize;

use fetiche_sources::{makepath, Auth};

const CONFIG: &str = "config.hcl";
const CVERSION: usize = 1;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub version: usize,
    pub sites: BTreeMap<String, Site>,
}

#[derive(Debug, Deserialize)]
pub struct Site {
    auth: Auth,
}

impl Default for Site {
    fn default() -> Self {
        Site { auth: Auth::Anon }
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

    pub fn load(fname: Option<String>) -> Result<Config> {
        let fname = match fname {
            Some(fname) => fname,
            _ => Self::default_file(),
        };
        let data = fs::read_to_string(fname).expect("Can not open config.hcl");

        match hcl::from_str(&data) {
            Ok(cfg) => {
                if cfg.version != CVERSION {
                    return Err(anyhow!("bad file version: {}", cfg.version));
                }
                Ok(cfg)
            }
            Err(e) => Err(anyhow!("bad configuration file: {}", e.to_string())),
        }
    }
}
