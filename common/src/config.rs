//! This is the `ConfigFile` struct.
//!
//! This is for finding the right default locations for various configuration files for
//! `fetiche`.  This is a configuration file/struct neutral loading engine, storing only the
//! base directory and with `load()` read the proper file or the default one.
//!
//! This encapsulates the configuration file, available with `.inner()` or `.inner_mut()`.
//!

use crate::{makepath, IntoConfig};

use directories::BaseDirs;
use eyre::{eyre, Result};
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
pub struct ConfigFile<T: Debug + DeserializeOwned + IntoConfig> {
    /// Tag is the project name.
    tag: String,
    /// This is the base directory for all files.
    basedir: PathBuf,
    inner: Option<T>,
}

impl<T> ConfigFile<T>
where
    T: Debug + DeserializeOwned + IntoConfig,
{
    #[tracing::instrument]
    fn new(tag: &str) -> Self {
        let base = BaseDirs::new();

        let basedir: PathBuf = match base {
            Some(base) => {
                #[cfg(unix)]
                let base = base.home_dir().join(".config").to_string_lossy().to_string();

                #[cfg(windows)]
                let base = base.data_local_dir().to_string_lossy().to_string();

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
        ConfigFile {
            tag: String::from(tag),
            basedir,
            inner: None,
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

    /// Load the file and return a struct T in the right format.
    ///
    /// Use the following search path:
    /// - default basedir (base on $HOME or $LOCALAPPDATA)
    /// - file specified on CLI
    ///
    #[tracing::instrument]
    pub fn load(fname: Option<&str>) -> Result<ConfigFile<T>> {
        // Create context
        //
        // FIXME: TAG is hardcoded.
        //
        let mut cfg = ConfigFile::<T>::new(TAG);

        let fname = match fname {
            Some(fname) => PathBuf::from(fname),
            None => cfg.default_file(),
        };

        // Use a full path
        //
        let fname = if fname.exists() {
            PathBuf::from(fname).canonicalize()?
        } else {
            return Err(eyre!("Unknown config file {:?} and no default in {:?}", fname, cfg.default_file()));
        };

        trace!("Loading config file {fname:?} from {:?}", cfg.config_path());

        let data = fs::read_to_string(fname)?;
        debug!("string data = {data}");

        let data: T = hcl::from_str(&data)?;
        debug!("struct data = {data:?}");

        cfg.inner = Some(data);
        Ok(cfg)
    }

    /// Return the inner configuration file
    ///
    pub fn inner(&self) -> &T {
        self.inner.as_ref().unwrap()
    }

    /// Return the inner configuration file as putable
    ///
    pub fn inner_mut(&mut self) -> &mut T {
        self.inner.as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IntoConfig, Versioned};
    use fetiche_macros::into_configfile;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

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
    #[into_configfile]
    #[derive(Debug, Default, Deserialize)]
    struct Bar {
        /// Each site credentials
        pub site: BTreeMap<String, Auth>,
    }

    pub const CVERSION: usize = 1;

    #[test]
    fn test_config_engine_load_default() -> Result<()> {
        // Explicitly load default
        //
        let cfg = ConfigFile::<Bar>::load(None)?;
        dbg!(&cfg);
        let inner = cfg.inner().unwrap();
        assert_eq!(CVERSION, inner.version());
        Ok(())
    }

    #[into_configfile]
    #[derive(Clone, Debug, Default, Deserialize)]
    struct Foo {
        pub name: String,
    }

    #[test]
    fn test_config_engine_load_file() -> Result<()> {
        let cfg = ConfigFile::<Foo>::load(Some("examples/local.hcl"))?;
        dbg!(&cfg);
        let inner = cfg.inner().unwrap();
        assert_eq!(CVERSION, inner.version());
        Ok(())
    }
}
