//! Main configuration management and loading
//!
use std::collections::hash_map::{IntoValues, Iter, Keys, Values, ValuesMut};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::create_dir_all;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;
use std::{env, fs};

use anyhow::Result;
use log::trace;
use serde::{Deserialize, Serialize};

use crate::Site;

#[cfg(unix)]
use home::home_dir;

/// Default configuration filename
const CONFIG: &str = "config.hcl";
const CVERSION: usize = 1;

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Main struct holding configurations
///
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Sites {
    version: usize,
    site: HashMap<String, Site>,
}

/// `Default` is for `unwrap_or_default()`.
///
impl Default for Sites {
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

impl Sites {
    /// Returns an empty struct
    ///
    #[inline]
    pub fn new() -> Sites {
        Sites {
            version: CVERSION,
            site: HashMap::<String, Site>::new(),
        }
    }

    /// Wrap `HashMap::get`
    ///
    #[inline]
    pub fn get(&self, name: &str) -> Option<&Site> {
        self.site.get(name)
    }

    /// Wrap `is_empty()`
    ///
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.site.is_empty()
    }

    /// Wrap `len()`
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.site.len()
    }

    /// Wrap `keys()`
    ///
    #[inline]
    pub fn keys(&self) -> Keys<'_, String, Site> {
        self.site.keys()
    }

    /// Wrap `index_mut()`
    ///
    #[inline]
    pub fn index_mut(&mut self, s: &str) -> Option<&Site> {
        self.site.get(s)
    }

    /// Wrap `values()`
    ///
    #[inline]
    pub fn values(&self) -> Values<'_, String, Site> {
        self.site.values()
    }

    /// Wrap `values_mut()`
    ///
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, String, Site> {
        self.site.values_mut()
    }

    /// Wrap `into_values()`
    ///
    #[inline]
    pub fn into_values(self) -> IntoValues<String, Site> {
        self.site.into_values()
    }

    /// Wrap `contains_key()`
    ///
    #[inline]
    pub fn contains_key(&self, s: &str) -> bool {
        self.site.contains_key(s)
    }

    /// Wrap `contains_key()`
    ///
    #[inline]
    pub fn iter(&self) -> Iter<'_, String, Site> {
        self.site.iter()
    }

    /// Load the specified config file
    ///
    fn read_file(fname: &PathBuf) -> Result<Sites> {
        trace!("Reading {:?}", fname);
        let content = fs::read_to_string(fname)?;

        // Check extension
        //
        let ext = match fname.extension() {
            Some(ext) => ext,
            _ => OsStr::new("hcl"),
        };

        trace!("File is .{ext:?}");
        let s: Sites = hcl::from_str(&content)?;
        if s.version != CVERSION {
            return Err(anyhow!("bad config version"));
        }
        Ok(s)
    }

    /// Returns the path of the default config file
    ///
    #[cfg(unix)]
    pub fn default_file() -> PathBuf {
        let homedir = home_dir().unwrap();
        let def: PathBuf = makepath!(homedir, BASEDIR, "drone-utils", CONFIG);
        trace!("Default file: {:?}", def);
        def
    }

    /// Returns the path of the default config file
    ///
    #[cfg(windows)]
    pub fn default_file() -> PathBuf {
        let homedir = env::var("LOCALAPPDATA").unwrap();

        let def: PathBuf = makepath!(homedir, "drone-utils", CONFIG);
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

        // Copy content of `config.hcl`  into place.
        //
        let fname: PathBuf = makepath!(&dir, CONFIG);
        let content = include_str!("config.hcl");
        fs::write(fname, content)
    }

    /// Load configuration from either the specified file or the default one.
    ///
    pub fn load(fname: &Option<PathBuf>) -> Result<Sites> {
        // Load default config if nothing is specified
        //
        match fname {
            // We have a configuration file
            //
            Some(cnf) => {
                trace!("Loading from {:?}", cnf);

                Sites::read_file(cnf)
            }
            // Need to load our own
            //
            _ => {
                let cnf = Sites::default_file();
                trace!("Loading from {:?}", cnf);

                Sites::read_file(&cnf)
            }
        }
    }
}

impl<'a> IntoIterator for &'a Sites {
    type Item = (&'a String, &'a Site);
    type IntoIter = Iter<'a, String, Site>;

    /// We can now do `sites.iter()`
    ///
    fn into_iter(self) -> Iter<'a, String, Site> {
        self.site.iter()
    }
}

impl Index<&str> for Sites {
    type Output = Site;

    /// Wrap `index()`
    ///
    #[inline]
    fn index(&self, s: &str) -> &Self::Output {
        let me = self.site.get(s);
        me.unwrap()
    }
}

impl IndexMut<&str> for Sites {
    /// Wrap `index_mut()`
    ///
    #[inline]
    fn index_mut(&mut self, s: &str) -> &mut Self::Output {
        let me = self.site.get_mut(s);
        if me.is_none() {
            self.site.insert(s.to_string(), Site::new());
        }
        self.site.get_mut(s).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Auth;
    use anyhow::bail;
    use std::env::temp_dir;

    #[test]
    fn test_new() {
        let a = Sites::new();
        assert!(a.is_empty());
        dbg!(&a);
    }

    #[test]
    fn test_config_load_hcl() {
        let cn: PathBuf = makepath!("src", CONFIG);
        assert!(cn.try_exists().is_ok());

        let cfg = Sites::read_file(&cn);
        dbg!(&cfg);
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        dbg!(&cfg);
        assert!(!cfg.is_empty());
        if let Some(site) = cfg.get("eih") {
            assert_eq!("http://127.0.0.1:2400", site.base_url);
            match &site.auth {
                Some(auth) => match auth {
                    Auth::Token {
                        password, token, ..
                    } => {
                        assert_eq!("NOPE", password);
                        assert_eq!("/login", token);
                    }
                    _ => panic!("foo"),
                },
                _ => (),
            }
        }
    }

    #[test]
    fn test_install_files() -> Result<()> {
        let tempdir = temp_dir();
        dbg!(&tempdir);
        let r = Sites::install_defaults(&tempdir);
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
