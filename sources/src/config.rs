//! Main configuration management and loading
//!
use std::collections::btree_map::{IntoValues, Iter, Keys, Values, ValuesMut};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::fs::create_dir_all;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
#[cfg(unix)]
use home::home_dir;
use log::{debug, trace};
use serde::{Deserialize, Serialize};

use crate::{makepath, Site};

/// Default configuration filename
const CONFIG: &str = "sources.hcl";
const CVERSION: usize = 3;

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// List of sources, this is the only exposed struct from here.
///
#[derive(Debug)]
pub struct Sources(BTreeMap<String, Site>);

impl Sources {
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
        let homedir = env!("LOCALAPPDATA");

        let def: PathBuf = makepath!(homedir, "drone-utils", CONFIG);
        def
    }

    /// Install default files
    ///
    pub fn install_defaults(dir: &PathBuf) -> std::io::Result<()> {
        // Create config directory if needed
        //
        if !dir.exists() {
            create_dir_all(dir)?
        }

        // Copy content of `sources.hcl`  into place.
        //
        let fname: PathBuf = makepath!(&dir, CONFIG);
        let content = include_str!("sources.hcl");
        fs::write(fname, content)
    }

    /// Load configuration from either the specified file or the default one.
    ///
    pub fn load(fname: &Option<PathBuf>) -> Result<Sources> {
        // Load default config if nothing is specified
        //
        let cnf = match fname {
            // We have a configuration file
            //
            Some(cnf) => {
                trace!("Loading from {:?}", cnf);
                cnf.into()
            }
            // Need to load our own
            //
            _ => {
                let cnf = Sources::default_file();
                trace!("Loading from {:?}", cnf);
                cnf
            }
        };
        let s = Sites::read_file(&cnf)?;
        let mut sources: BTreeMap<String, Site> = BTreeMap::new();

        s.iter().for_each(|s| {
            let key = s.name.clone().unwrap();

            sources.insert(key, s.clone());
        });
        Ok(Sources(sources))
    }

    /// Wrap `get`
    ///
    #[inline]
    pub fn get(&self, name: &str) -> Option<&Site> {
        self.0.get(name)
    }

    /// Wrap `get_mut`
    ///
    #[inline]
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Site> {
        self.0.get_mut(name)
    }

    /// Wrap `is_empty()`
    ///
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Wrap `len()`
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Wrap `keys()`
    ///
    #[inline]
    pub fn keys(&self) -> Keys<'_, String, Site> {
        self.0.keys()
    }

    /// Wrap `index()`
    ///
    #[inline]
    pub fn index(&self, s: &str) -> Option<&Site> {
        self.0.get(s)
    }

    /// Wrap `index_mut()`
    ///
    #[inline]
    pub fn index_mut(&mut self, s: &str) -> Option<&Site> {
        self.0.get(s)
    }

    /// Wrap `values()`
    ///
    #[inline]
    pub fn values(&self) -> Values<'_, String, Site> {
        self.0.values()
    }

    /// Wrap `values_mut()`
    ///
    #[inline]
    pub fn values_mut(&mut self) -> ValuesMut<'_, String, Site> {
        self.0.values_mut()
    }

    /// Wrap `into_values()`
    ///
    #[inline]
    pub fn into_values(self) -> IntoValues<String, Site> {
        self.0.into_values()
    }

    /// Wrap `contains_key()`
    ///
    #[inline]
    pub fn contains_key(&self, s: &str) -> bool {
        self.0.contains_key(s)
    }

    /// Wrap `contains_key()`
    ///
    #[inline]
    pub fn iter(&self) -> Iter<'_, String, Site> {
        self.0.iter()
    }
}

impl Index<&str> for Sources {
    type Output = Site;

    /// Wrap `index()`
    ///
    #[inline]
    fn index(&self, s: &str) -> &Self::Output {
        self.0.get(s).unwrap()
    }
}

impl Index<String> for Sources {
    type Output = Site;

    /// Wrap `index()`
    ///
    #[inline]
    fn index(&self, s: String) -> &Self::Output {
        self.0.get(&s).unwrap()
    }
}

impl IndexMut<&str> for Sources {
    /// Wrap `index_mut()`
    ///
    #[inline]
    fn index_mut(&mut self, s: &str) -> &mut Self::Output {
        let me = self.0.get_mut(s);
        if me.is_none() {
            self.0.insert(s.to_string(), Site::new());
        }
        self.0.get_mut(s).unwrap()
    }
}

impl IndexMut<String> for Sources {
    /// Wrap `index_mut()`
    ///
    #[inline]
    fn index_mut(&mut self, s: String) -> &mut Self::Output {
        let me = self.0.get_mut(&s);
        if me.is_none() {
            self.0.insert(s.to_string(), Site::new());
        }
        self.0.get_mut(&s).unwrap()
    }
}

impl<'a> IntoIterator for &'a Sources {
    type Item = (&'a String, &'a Site);
    type IntoIter = Iter<'a, String, Site>;

    /// We can now do `sources.iter()`
    ///
    fn into_iter(self) -> Iter<'a, String, Site> {
        self.0.iter()
    }
}

// -----

/// Main struct holding configurations internally
///
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Sites {
    version: usize,
    site: BTreeMap<String, Site>,
}

/// `Default` is for `unwrap_or_default()`.
///
impl Default for Sites {
    fn default() -> Self {
        Self::new()
    }
}

impl Sites {
    /// Returns an empty struct
    ///
    #[inline]
    pub fn new() -> Sites {
        Sites {
            version: CVERSION,
            site: BTreeMap::<String, Site>::new(),
        }
    }

    /// Load the specified config file
    ///
    fn read_file(fname: &PathBuf) -> Result<Vec<Site>> {
        trace!("Reading {:?}", fname);
        let content = fs::read_to_string(fname)?;

        // Check extension
        //
        let ext = match fname.extension() {
            Some(ext) => ext,
            _ => OsStr::new("hcl"),
        };

        debug!("File is .{ext:?}");
        let s: Sites = hcl::from_str(&content)?;

        // First check
        //
        if s.version != CVERSION {
            return Err(anyhow!("bad config version"));
        }

        // Fetch the site name and insert it into each Site
        //
        let s: Vec<_> = s
            .site
            .keys()
            .map(|n| {
                let site = s.site.get(n).unwrap();
                Site {
                    dtype: site.dtype,
                    name: Some(n.clone()),
                    format: site.format.clone(),
                    auth: site.auth.clone(),
                    base_url: site.base_url.clone(),
                    routes: site.routes.clone(),
                }
            })
            .collect();

        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use crate::DataType;
    use anyhow::bail;

    use crate::site::Auth;

    use super::*;

    #[test]
    fn test_sites_load_hcl() {
        let cn: PathBuf = makepath!("src", "sources.hcl");
        assert!(cn.try_exists().is_ok());

        let cfg = Sources::load(&Some(cn));
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        assert!(!cfg.is_empty());
        assert_eq!(5, cfg.len());

        // Check one
        //
        if let Some(site) = cfg.get("eih") {
            assert_eq!("http://127.0.0.1:2400", site.base_url);
            assert_eq!(DataType::Drone, site.dtype);
            match &site.auth {
                Some(auth) => match auth {
                    Auth::Token {
                        password, token, ..
                    } => {
                        assert_eq!("NOPE", password);
                        assert_eq!("/login", token);
                    }
                    _ => panic!("bad auth"),
                },
                _ => (),
            }
        }

        // Check another one
        //
        if let Some(site) = cfg.get("opensky") {
            assert_eq!("https://opensky-network.org/api", site.base_url);
            assert_eq!(DataType::Adsb, site.dtype);
            match &site.auth {
                Some(auth) => match auth {
                    Auth::Login {
                        username, password, ..
                    } => {
                        assert_eq!("dphu", username);
                        assert_eq!("NOPE", password);
                    }
                    _ => panic!("bad auth"),
                },
                _ => (),
            }
        }
    }

    #[test]
    fn test_install_files() -> Result<()> {
        let tempdir = temp_dir();
        debug!("{:?}", tempdir);

        match Sources::install_defaults(&tempdir) {
            Ok(()) => {
                let f: PathBuf = makepath!(tempdir, CONFIG);
                assert!(f.exists());
            }
            _ => bail!("all failed"),
        }
        Ok(())
    }
}
