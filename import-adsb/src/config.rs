//! Main configuration management and loading
//!
//! This is mainly the database connection string that is needed.
//!
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};

use anyhow::Result;
use clap::crate_name;
#[cfg(unix)]
use home::home_dir;
use log::{debug, trace};
use serde::{Deserialize, Serialize};

/// Default configuration filename
const CONFIG: &str = "dbfile.hcl";
const DVERSION: usize = 1;

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// `sqlx` support all these
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DB {
    MySQL {
        host: String,
        user: String,
        url: String,
        tls: bool,
    },
    Influx {
        host: String,
        org: String,
        token: String,
    },
    Pgsql {
        url: String,
    },
    SQLite {
        path: String,
    },
}

/// Main struct holding configurations
///
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DBFile {
    /// Config file versioning
    pub version: usize,
    /// Default format-specs
    pub default: String,
    /// Site map
    pub db: HashMap<String, DB>,
}

/// `Default` is for `unwrap_or_default()`.
///
impl Default for DBFile {
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

impl DBFile {
    /// Returns an empty struct
    ///
    pub fn new() -> DBFile {
        let h = HashMap::<String, DB>::new();
        DBFile {
            version: DVERSION,
            default: "none".to_string(),
            db: h,
        }
    }

    /// Load the specified config file
    ///
    pub fn load(fname: &PathBuf) -> Result<DBFile> {
        trace!("Reading {:?}", fname);
        let content = fs::read_to_string(fname)?;
        let s: DBFile = hcl::from_str(&content)?;
        debug!("{:?}", s);
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
}

/// Load configuration from either the specified file or the default one.
///
pub fn get_config(fname: &Option<PathBuf>) -> DBFile {
    // Load default config if nothing is specified
    //
    match fname {
        // We have a configuration file
        //
        Some(cnf) => {
            trace!("Loading from {:?}", cnf);

            DBFile::load(cnf).unwrap_or_else(|_| panic!("No file {:?}", cnf))
        }
        // Need to load our own
        //
        None => {
            let cnf = DBFile::default_file();
            trace!("Loading from {:?}", cnf);

            DBFile::load(&cnf).unwrap_or_else(|_| panic!("No default file {:?}", cnf))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let a = DBFile::new();
        assert_eq!("none", a.default);
        assert!(a.db.is_empty());
        dbg!(&a);
    }

    #[test]
    fn test_config_load() {
        let cn: PathBuf = makepath!("..", "import-adsb", "src", CONFIG);
        assert!(cn.try_exists().is_ok());
        dbg!(&cn);

        let cfg = DBFile::load(&cn);
        dbg!(&cfg);
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        assert!(!cfg.db.is_empty());
        let someplace = &cfg.db["local"];
        assert_eq!(DVERSION, cfg.version);
        match someplace {
            DB::SQLite { path, .. } => assert_eq!("testdata/adsb.sqlite", path),
            _ => (),
        }
    }

    #[test]
    fn test_serialize_db() {
        let dbfile = DBFile {
            version: 1,
            default: "foo".to_string(),
            db: HashMap::<String, DB>::from([
                (
                    "foo".to_string(),
                    DB::MySQL {
                        user: "root".to_string(),
                        host: "mysql.db.local".to_string(),
                        tls: true,
                        url: "mysql://foo.example.net".to_string(),
                    },
                ),
                (
                    "local".to_string(),
                    DB::SQLite {
                        path: "testdata/adsb.sqlite".to_string(),
                    },
                ),
            ]),
        };
        let local = &dbfile.db["local"];
        assert_eq!(
            &DB::SQLite {
                path: "testdata/adsb.sqlite".to_string(),
            },
            local
        );
    }
}
