//! This is the `ConfigEngine` struct.
//!
//! This is for finding the right default locations for various configuration files for
//! `fetiche`.  This is a a configuration file/struct neutral loading engine, storing only the
//! base directory and with `load()` read the proper file or the default one.
//!

use crate::{makepath, Versioned};

use directories::BaseDirs;
use eyre::Result;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::{env, fs};
use tracing::{debug, error, trace};

/// Config filename
const CONFIG: &str = "config.hcl";

/// Main name for the directory base
const TAG: &str = "drone-utils";

/// Configuration for the CLI tool, supposed to include parameters and most importantly
/// credentials for the various sources.
///
#[derive(Debug)]
pub struct ConfigEngine<T: Debug + DeserializeOwned + Versioned> {
    /// Version in the file MUST match `CVERSION`
    tag: String,
    basedir: PathBuf,
    _a: PhantomData<T>,
}

impl<T> ConfigEngine<T>
where
    T: Debug + DeserializeOwned + Versioned,
{
    #[tracing::instrument]
    fn new(tag: &str) -> Self {
        let base = BaseDirs::new();

        let basedir: PathBuf = match base {
            Some(base) => {
                let base = base.config_local_dir().to_string_lossy().to_string();
                debug!("base = {base}");
                let base: PathBuf = makepath!(base, tag);
                base
            }
            None => {
                #[cfg(unix)]
                let homedir = std::env::var("HOME")
                    .map_err(|_| error!("No HOME variable defined, can not continue"))
                    .unwrap();

                #[cfg(windows)]
                let homedir = env::var("LOCALAPPDATA")
                    .map_err(|_| error!("No LOCALAPPDATA variable defined, can not continue"))
                    .unwrap();

                debug!("base = {homedir}");

                #[cfg(unix)]
                let base: PathBuf = makepath!(homedir, ".config", tag);

                #[cfg(windows)]
                let base: PathBuf = makepath!(homedir, tag);

                base
            }
        };
        ConfigEngine {
            tag: String::from(tag),
            basedir,
            _a: PhantomData,
        }
    }

    /// Returns the path of the default config directory
    ///
    #[tracing::instrument]
    pub fn config_path(&self) -> PathBuf {
        self.basedir.clone()
    }

    /// Returns the path of the default config file
    ///
    #[tracing::instrument]
    pub fn default_file(&self) -> PathBuf {
        let cfg = self.config_path().join(CONFIG);
        debug!("default = {cfg:?}");
        cfg
    }

    #[tracing::instrument]
    pub fn load(fname: Option<&str>) -> Result<T> {
        trace!("loading config");

        let cfg = ConfigEngine::<T>::new(TAG);

        trace!("Loading {fname:?}");
        let fname = match fname {
            Some(fname) => PathBuf::from(fname),
            None => cfg.default_file(),
        };

        let data = fs::read_to_string(fname)?;
        debug!("string data = {data}");

        let data: T = hcl::from_str(&data)?;
        debug!("struct data = {data:?}");

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Clone, Debug, Deserialize)]
    struct Foo {
        version: usize,
        pub name: String,
    }

    impl Versioned for Foo {
        fn version(&self) -> usize {
            self.version
        }
    }

    /// Describe the possible ways to authenticate oneself
    ///
    #[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum Auth {
        /// Nothing special, no auth
        #[default]
        Anon,
        /// Using an API key supplied through the URL or a header
        Key { api_key: String },
        /// Using a login/passwd to get a token
        Token {
            login: String,
            password: String,
            token: String,
        },
        /// Using plain login/password
        Login { username: String, password: String },
    }

    /// Configuration for the CLI tool, supposed to include parameters and most importantly
    /// credentials for the various sources.
    ///
    #[derive(Debug, Deserialize)]
    struct ConfigFile {
        /// Version in the file MUST match `CVERSION`
        pub version: usize,
        /// Each site credentials
        pub site: BTreeMap<String, Auth>,
    }

    pub const CVERSION: usize = 1;

    impl Versioned for ConfigFile {
        fn version(&self) -> usize {
            CVERSION
        }
    }

    #[test]
    fn test_configengine_load_default() -> Result<()> {
        // Explicitly load default
        //
        let cfg: ConfigFile = ConfigEngine::load(None)?;
        dbg!(&cfg);
        assert_eq!(CVERSION, cfg.version);
        Ok(())
    }

    #[test]
    fn test_configengine_load_file() -> Result<()> {
        // Explicitely load default
        //
        let cfg: Foo = ConfigEngine::load(Some("examples/local.hcl"))?;
        dbg!(&cfg);
        assert_eq!(CVERSION, cfg.version);
        Ok(())
    }
}
