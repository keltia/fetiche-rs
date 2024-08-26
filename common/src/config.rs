//! This is the `ConfigFile` struct.
//!
//! This is for finding the right default locations for various configuration files for
//! `fetiche`.  This is a configuration file/struct neutral loading engine, storing only the
//! base directory and with `load()` read the proper file or the default one.
//!
//! This encapsulates the configuration file, available with `.inner()` or `.inner_mut()`.
//!

use crate::IntoConfig;

use directories::BaseDirs;
use eyre::{eyre, Result};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::fs::read_dir;
use std::path::{Path, PathBuf};
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
    root: PathBuf,
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
                let base = base.home_dir().join(".config");

                #[cfg(windows)]
                let base = base.data_local_dir();

                debug!("base = {base:?}");
                let base = base.join(Path::new(tag));
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
                let base = Path::new(&homedir)
                    .join(Path::new(".config"))
                    .join(Path::new(tag));

                #[cfg(windows)]
                let base = PathBuf::from(homedir).join(tag);

                base
            }
        };
        ConfigFile {
            tag: String::from(tag),
            root: basedir,
            inner: None,
        }
    }

    /// Return the project tag
    ///
    pub fn tag(&self) -> String {
        self.tag.clone()
    }

    /// Returns the path of the default config directory
    ///
    #[tracing::instrument(skip(self))]
    pub fn config_path(&self) -> PathBuf {
        self.root.clone()
    }

    /// Returns the path of the default config file
    ///
    #[tracing::instrument(skip(self))]
    pub fn default_file(&self) -> String {
        let f = String::from(CONFIG);
        trace!("Default filename: {f}");
        f
    }

    /// Return our root
    ///
    #[tracing::instrument(skip(self))]
    pub fn root(&self) -> PathBuf {
        self.root.clone()
    }

    /// Load the file and return a struct T in the right format.
    ///
    /// Use the following search path:
    /// - default basedir (base on $HOME or $LOCALAPPDATA)
    /// - file specified on CLI
    ///
    /// Example:
    /// ```no_run
    /// use serde::Deserialize;
    /// use fetiche_common::{ConfigFile, IntoConfig, Versioned};
    ///
    /// use fetiche_macros::into_configfile;
    ///
    /// #[into_configfile]
    /// #[derive(Debug, Default, Deserialize)]
    /// struct Foo {
    ///     // We need at least one named field
    ///     a: u32,
    /// }
    ///
    /// // This will load "config.hcl" from the base directory
    /// //
    /// let cfg = ConfigFile::<Foo>::load(None).unwrap();
    ///
    /// // Access the loaded configuration file
    /// //
    /// let conf = cfg.inner();
    /// ```
    ///
    /// NOTE: if `fname`
    #[tracing::instrument]
    pub fn load(fname: Option<&str>) -> Result<ConfigFile<T>> {
        // Create context
        //
        // FIXME: TAG is hardcoded.
        //
        let mut cfg = ConfigFile::<T>::new(TAG);

        // Check is None was passed to get the default file from the default location:
        //
        let fname = if fname.is_none() {
            let def = PathBuf::from(cfg.default_file()).canonicalize()?;
            dbg!(&def);
            def
        } else {
            // Do we have a bare filename?
            //
            let fname = fname.unwrap();
            let p = PathBuf::from(fname);

            if p.file_name().unwrap() == p {
                cfg.root.join(fname).canonicalize()?
            } else {
                // If it is relative or absolute, assume it exists and return its canonical form
                //
                PathBuf::from(fname).canonicalize()?
            }
        };
        assert!(fname.is_absolute());

        trace!("Loading config file {fname:?} from {:?}", cfg.config_path());

        let data = match fs::read_to_string(&fname) {
            Ok(data) => data,
            Err(e) => {
                return Err(eyre!("Error: failed to read config file {fname:?}: {e}"));
            }
        };
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

    /// Return the inner configuration file as mutable
    ///
    pub fn inner_mut(&mut self) -> &mut T {
        self.inner.as_mut().unwrap()
    }

    /// Return the list of possible configuration files within basedir
    ///
    pub fn list(&self) -> Vec<String> {
        env::set_current_dir(self.root.as_path()).unwrap();

        if let Ok(dir) = read_dir(env::current_dir().unwrap()) {
            let list = dir
                .into_iter()
                .filter_map(|f| {
                    if let Ok(p) = f {
                        if p.file_name().to_string_lossy().ends_with(".hcl") {
                            Some(p.file_name().into_string().unwrap())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            return list;
        }
        vec![]
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
    enum Auth {
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
        let cfg = ConfigFile::<Bar>::load(Some("config.hcl"));
        assert!(cfg.is_ok());
        let cfg = cfg?;
        let inner = cfg.inner();
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
        let cfg = ConfigFile::<Foo>::load(Some("examples/local.hcl"));
        assert!(cfg.is_ok());
        let cfg = cfg?;
        let inner = cfg.inner();
        assert_eq!(CVERSION, inner.version());
        Ok(())
    }
}
