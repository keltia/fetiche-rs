//! Main configuration management and loading
//!
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};

use anyhow::{Context, Result};
use clap::crate_name;
use serde::{Deserialize, Serialize};

use crate::format::Source;
use crate::site::Site;

#[cfg(unix)]
use home::home_dir;

/// Default configuration filename
const CONFIG: &str = "config.toml";

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Main struct holding configurations
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Config {
    /// Default format
    pub default: String,
    /// Site map
    pub sites: HashMap<String, Site>,
}

/// `Default` is for `unwrap_or_default()`.
impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Returns an empty struct
    pub fn new() -> Config {
        let h = HashMap::<String, Site>::new();
        Config {
            default: "none".to_string(),
            sites: h,
        }
    }

    /// Load the specified config file
    pub fn load(fname: &PathBuf) -> Result<Config> {
        let content = fs::read_to_string(fname)?;

        let s: Config = toml::from_str(&content)?;
        Ok(s)
    }

    /// Returns the path of the default config file
    #[cfg(unix)]
    pub fn default_file() -> PathBuf {
        let homedir = home_dir().unwrap();
        let def: PathBuf = [
            homedir,
            PathBuf::from(BASEDIR),
            PathBuf::from(crate_name!()),
            PathBuf::from(CONFIG),
        ]
        .iter()
        .collect();
        def
    }

    /// Returns the path of the default config file
    #[cfg(windows)]
    pub fn default_file() -> PathBuf {
        let homedir = env::var("LOCALAPPDATA").unwrap();

        let def: PathBuf = [
            PathBuf::from(homedir),
            PathBuf::from(crate_name!()),
            PathBuf::from(CONFIG),
        ]
        .iter()
        .collect();
        def
    }
}

/// Load configuration from either the specified file or the default one.
///
pub fn get_config(fname: &Option<PathBuf>) -> Config {
    // Load default config if nothing is specified
    //
    let cfg = match fname {
        // We have a configuration file
        //
        Some(c) => Config::load(c).with_context(|| format!("No file {:?}", c)),
        // Need to load our own
        //
        None => {
            let cnf = Config::default_file();

            Config::load(&cnf).with_context(|| format!("No file {:?}", cnf))
        }
    };

    // We must have a valid configuration, an error means no default one
    match cfg {
        Ok(c) => c,
        Err(e) => panic!("Need a config file! {}", e),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let a = Config::new();
        assert_eq!("none", a.default);
        assert!(a.sites.is_empty());
        println!("{:?}", a)
    }

    #[test]
    fn test_config_load() {
        let cn = PathBuf::from("src/config.toml");
        let cfg = Config::load(&cn);
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        assert!(!cfg.sites.is_empty());
        let someplace = &cfg.sites["someplace"];
        match someplace {
            Site::Login { password, .. } => assert_eq!("NOPE", password),
            _ => (),
        }
    }
}
