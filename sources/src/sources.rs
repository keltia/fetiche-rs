//! This is the exposed part of the `fetiche-sources` API.
//!

use std::collections::btree_map::{IntoValues, Iter, Keys, Values, ValuesMut};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
#[cfg(unix)]
use home::home_dir;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::{makepath, Auth, Site, CONFIG, CVERSION, TOKEN_BASE};

/// List of sources, this is the only exposed struct from here.
///
#[derive(Debug)]
pub struct Sources(BTreeMap<String, Site>);

impl Sources {
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

    /// Install default files
    ///
    pub fn install_defaults(dir: &PathBuf) -> std::io::Result<()> {
        // Create config directory if needed
        //
        if !dir.exists() {
            fs::create_dir_all(dir)?
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

    /// List of currently known sources into a nicely formatted string.
    ///
    pub fn list(&self) -> Result<String> {
        let header = vec!["Name", "Type", "Format", "URL", "Auth"];

        let mut builder = Builder::default();
        builder.set_header(header);

        self.0.iter().for_each(|(n, s)| {
            let mut row = vec![];

            let dtype = s.dtype.clone().to_string();
            let format = s.format.clone();
            let base_url = s.base_url.clone();
            row.push(n);
            row.push(&dtype);
            row.push(&format);
            row.push(&base_url);
            let auth = if let Some(auth) = &s.auth {
                match auth {
                    Auth::Login { .. } => "login",
                    Auth::Token { .. } => "token",
                    Auth::Anon => "open",
                    Auth::Key { .. } => "API key",
                }
                .to_string()
            } else {
                "anon".to_owned()
            };
            row.push(&auth);
            builder.push_record(row);
        });

        let table = builder.build().with(Style::rounded()).to_string();
        let table = format!("Listing all sources:\n\n{table}");
        Ok(table)
    }
}

// Token management
//
impl Sources {
    /// Returns the path of the directory storing tokens
    ///
    pub fn token_path() -> PathBuf {
        Self::config_path().join(TOKEN_BASE)
    }

    /// Return the content of named token
    ///
    pub fn get_token(name: &str) -> Result<String> {
        let t = Self::token_path().join(name);
        trace!("get_token: {t:?}");
        if t.exists() {
            Ok(fs::read_to_string(t)?)
        } else {
            Err(anyhow!("{:?}: No such file", t))
        }
    }

    /// Store (overwrite) named token
    ///
    pub fn store_token(name: &str, data: &str) -> Result<()> {
        let p = Self::token_path();

        // Check token cache
        //
        if !p.exists() {
            // Create it
            //
            trace!("create token store: {p:?}");

            fs::create_dir_all(p)?
        }
        let t = Self::token_path().join(name);
        trace!("store_token: {t:?}");
        Ok(fs::write(t, data)?)
    }

    /// Purge expired token
    ///
    pub fn purge_token(name: &str) -> Result<()> {
        trace!("purge expired token");
        let p = Self::token_path().join(name);
        Ok(fs::remove_file(p)?)
    }

    /// List tokens
    ///
    pub fn list_tokens() -> Result<String> {
        trace!("listing tokens");

        let header = vec!["Path", "Created at"];

        let mut builder = Builder::default();
        builder.set_header(header);

        let p = Self::token_path();
        if let Ok(dir) = fs::read_dir(p) {
            for fname in dir {
                let mut row = vec![];

                if let Ok(fname) = fname {
                    // Using strings is easier
                    //
                    let name = format!("{}", fname.file_name().to_string_lossy());
                    row.push(name.clone());

                    let st = fname.metadata().unwrap();
                    let modified = DateTime::<Utc>::from(st.modified().unwrap());
                    let modified = format!("{}", modified);
                    row.push(modified);
                } else {
                    row.push("INVALID".to_string());
                    let origin = format!("{}", DateTime::<Utc>::from(UNIX_EPOCH));
                    row.push(origin);
                }
                builder.push_record(row);
            }
        }
        let table = builder.build().with(Style::rounded()).to_string();
        let table = format!("Listing all tokens:\n{}", table);
        Ok(table)
    }
}

// -----

/// Helper methods
///
impl Sources {
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
        let s: hcl::error::Result<Sites> = hcl::from_str(&content);
        let s = match s {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("syntax error or wrong version: {}", e)),
        };

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

    use anyhow::bail;
    use log::debug;

    use crate::site::Auth;
    use crate::DataType;

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

    #[test]
    #[cfg(unix)]
    fn test_basedir() {
        let p = Sources::config_path();
        let ep: PathBuf = makepath!(env!("HOME"), BASEDIR, "drone-utils");
        assert_eq!(ep, p);
    }

    #[test]
    #[cfg(windows)]
    fn test_basedir() {
        let p = Sources::config_path();
        let ep: PathBuf = makepath!(env!("LOCALAPPDATA"), "drone-utils");
        assert_eq!(ep, p);
    }

    #[test]
    #[cfg(unix)]
    fn test_token_path() {
        let p = Sources::token_path();
        let ep: PathBuf = makepath!(env!("HOME"), BASEDIR, "drone-utils", "tokens");
        dbg!(Sources::config_path());
        assert_eq!(ep, p);
    }
}
