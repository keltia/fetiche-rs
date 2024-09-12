//! This is the exposed part of the `fetiche-sources` API.
//!
//! FIXME: too many dependencies on being part of the binary and not from `fetiched`.
//!

use std::collections::btree_map::{IntoValues, Iter, IterMut, Keys, Values, ValuesMut};
use std::collections::BTreeMap;
use std::fs;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;

use eyre::Result;
use serde::Deserialize;
use tabled::builder::Builder;
use tabled::settings::Style;

use crate::{Auth, Site, CONFIG};

use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;

/// List of sources, this is the only exposed struct from here.
///
#[into_configfile(version = 4, filename = "sources.hcl")]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SourcesConfig {
    site: BTreeMap<String, Site>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Sources {
    site: BTreeMap<String, Site>,
}

/// Initialise a `Source` from a `BTreeMap`
///
impl From<BTreeMap<String, Site>> for Sources {
    fn from(value: BTreeMap<String, Site>) -> Self {
        Sources {
            site: value.clone(),
        }
    }
}

/// Initialise a `Source` from a `BTreeMap`
///
impl From<Vec<(String, Site)>> for Sources {
    fn from(value: Vec<(String, Site)>) -> Self {
        let mut sites = BTreeMap::<String, Site>::new();
        value.iter().for_each(|(n, s)| {
            sites.insert(n.clone(), s.clone());
        });
        Sources { site: sites }
    }
}

impl Sources {
    #[tracing::instrument]
    pub fn load() -> Result<Self> {
        let src_file = ConfigFile::<SourcesConfig>::load(Some("sources.hcl"))?;
        let src = src_file.inner();

        let all = src
            .site
            .iter()
            .map(|(n, s)| {
                let mut site = s.clone();

                site.name = n.to_string();
                site.token_base = src_file.root();
                (n.to_string(), site)
            })
            .collect::<Vec<_>>();
        let s = Sources::from(all);
        Ok(s)
    }

    /// Install default files
    ///
    #[tracing::instrument]
    pub fn install_defaults(dir: &PathBuf) -> std::io::Result<()> {
        // Create config directory if needed
        //
        if !dir.exists() {
            fs::create_dir_all(dir)?
        }

        // Copy content of `sources.hcl`  into place.
        //
        let fname: PathBuf = dir.join(CONFIG);
        let content = include_str!("sources.hcl");
        fs::write(fname, content)
    }

    /// List of currently known sources into a nicely formatted string.
    ///
    #[tracing::instrument(skip(self))]
    pub fn list(&self) -> Result<String> {
        let header = vec!["Name", "Type", "Format", "URL", "Auth", "Ops"];

        let mut builder = Builder::default();
        builder.push_record(header);

        self.site.iter().for_each(|(n, s)| {
            let mut row = vec![];

            let dtype = s.dtype.clone().to_string();
            let format = s.format.clone().to_string();
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
                    Auth::UserKey { .. } => "API+User keys",
                }
                .to_string()
            } else {
                "anon".to_owned()
            };
            row.push(&auth);
            let cap = s
                .features
                .clone()
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<String>>()
                .join(",");
            row.push(&cap);
            builder.push_record(row);
        });

        let table = builder.build().with(Style::rounded()).to_string();
        let table = format!("Listing all sources:\n{table}");
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
        self.site.get(name)
    }

    /// Wrap `get_mut`
    ///
    #[inline]
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Site> {
        self.site.get_mut(name)
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

    /// Wrap `index()`
    ///
    #[inline]
    pub fn index(&self, s: &str) -> Option<&Site> {
        self.site.get(s)
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

    /// Wrap `iter()`
    ///
    #[inline]
    pub fn iter(&self) -> Iter<'_, String, Site> {
        self.site.iter()
    }

    /// Wrap `iter_mut()`
    ///
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, String, Site> {
        self.site.iter_mut()
    }
}

impl Index<&str> for Sources {
    type Output = Site;

    /// Wrap `index()`
    ///
    #[inline]
    fn index(&self, s: &str) -> &Self::Output {
        self.site.get(s).unwrap()
    }
}

impl Index<String> for Sources {
    type Output = Site;

    /// Wrap `index()`
    ///
    #[inline]
    fn index(&self, s: String) -> &Self::Output {
        self.site.get(&s).unwrap()
    }
}

impl IndexMut<&str> for Sources {
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

impl IndexMut<String> for Sources {
    /// Wrap `index_mut()`
    ///
    #[inline]
    fn index_mut(&mut self, s: String) -> &mut Self::Output {
        let me = self.site.get_mut(&s);
        if me.is_none() {
            self.site.insert(s.to_string(), Site::new());
        }
        self.site.get_mut(&s).unwrap()
    }
}

impl<'a> IntoIterator for &'a Sources {
    type Item = (&'a String, &'a Site);
    type IntoIter = Iter<'a, String, Site>;

    /// We can now do `sources.iter()`
    ///
    fn into_iter(self) -> Iter<'a, String, Site> {
        self.site.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use crate::DataType;
    use eyre::bail;
    use fetiche_common::ConfigFile;
    use tracing::debug;

    use super::*;

    #[test]
    fn test_sites_load_hcl() {
        let cn = PathBuf::from("src").join("sources.hcl");
        assert!(cn.try_exists().is_ok());

        let cfile = ConfigFile::<Sources>::load(Some(&cn.to_string_lossy().to_string()));
        assert!(cfile.is_ok());

        let cfg = cfile.unwrap().inner();
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
                let f = tempdir.join(CONFIG);
                assert!(f.exists());
            }
            _ => bail!("all failed"),
        }
        Ok(())
    }
}
