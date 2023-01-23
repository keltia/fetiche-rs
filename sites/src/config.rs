//! Main configuration management and loading
//!
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;
use std::{env, fs};

use anyhow::{anyhow, Result};
use clap::crate_name;
use log::trace;
use serde::{Deserialize, Serialize};

use crate::Site;

#[cfg(unix)]
use home::home_dir;

/// Default configuration filename
const CONFIG: &str = "config.toml";

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Main struct holding configurations
///
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Config {
    /// Default format-specs
    pub default: String,
    /// Site map
    pub sites: HashMap<String, Site>,
}

/// `Default` is for `unwrap_or_default()`.
///
impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple macro to generate PathBuf from a series of entries
///
#[macro_export]
macro_rules! makepath {
    ($($item:expr),+) => {
        [
        $(PathBuf::from($item),)+
        ]
        .iter()
        .collect()
    };
}

impl Config {
    /// Returns an empty struct
    ///
    pub fn new() -> Config {
        let h = HashMap::<String, Site>::new();
        Config {
            default: "none".to_string(),
            sites: h,
        }
    }

    /// Load the specified config file
    ///
    pub fn load(fname: &PathBuf) -> Result<Config> {
        trace!("Reading {:?}", fname);
        let content = fs::read_to_string(fname)?;

        let s: Config = toml::from_str(&content)?;
        Ok(s)
    }

    /// Returns the path of the default config file
    ///
    #[cfg(unix)]
    pub fn default_file() -> PathBuf {
        let homedir = home_dir().unwrap();
        let def: PathBuf = makepath!(homedir, BASEDIR, crate_name!(), CONFIG);
        trace!("Default file: {:?}", def);
        def
    }

    /// Returns the path of the default config file
    ///
    #[cfg(windows)]
    pub fn default_file() -> PathBuf {
        let homedir = env::var("LOCALAPPDATA").unwrap();

        let def: PathBuf = makepath!(homedir, crate_name!(), CONFIG);
        def
    }

    /// Install default files
    ///
    pub fn install_defaults(dir: &PathBuf) -> std::io::Result<()> {
        // Create config directory if needed
        //
        if !dir.exists() {
            create_dir_all(&dir)?
        }

        // Copy content of `config.toml`  into place.
        //
        let fname: PathBuf = makepath!(&dir, CONFIG);
        let content = include_str!("config.toml");
        fs::write(fname, content)
    }
}

/// Load configuration from either the specified file or the default one.
///
pub fn get_config(fname: &Option<PathBuf>) -> Config {
    // Load default config if nothing is specified
    //
    match fname {
        // We have a configuration file
        //
        Some(cnf) => {
            trace!("Loading from {:?}", cnf);

            Config::load(cnf).unwrap_or_else(|_| panic!("No file {:?}", cnf))
        }
        // Need to load our own
        //
        None => {
            let cnf = Config::default_file();
            trace!("Loading from {:?}", cnf);

            Config::load(&cnf).unwrap_or_else(|_| panic!("No default file {:?}", cnf))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::bail;
    use std::env::temp_dir;

    #[test]
    fn test_new() {
        let a = Config::new();
        assert_eq!("none", a.default);
        assert!(a.sites.is_empty());
        dbg!(&a);
    }

    #[test]
    fn test_config_load() {
        let cn: PathBuf = makepath!("src", "bin", "cat21conv", CONFIG);
        assert!(cn.try_exists().is_ok());

        let cfg = Config::load(&cn);
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        assert!(!cfg.sites.is_empty());
        let someplace = &cfg.sites["eih"];
        match someplace {
            Site::Login { password, .. } => assert_eq!("NOPE", password),
            _ => (),
        }
    }

    #[test]
    fn test_install_files() -> Result<()> {
        let tempdir = temp_dir();
        dbg!(&tempdir);
        let r = Config::install_defaults(&tempdir);
        match r {
            Ok(()) => {
                let f: PathBuf = makepath!(tempdir, CONFIG);
                assert!(f.exists());
            }
            _ => bail!("all failed"),
        }
        Ok(())
    }
}
