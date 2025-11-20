//! These are some of the support functions for the `Sources` struct.
//!

use std::collections::btree_map::{IntoValues, Iter, IterMut, Keys, Values, ValuesMut};
use std::collections::BTreeMap;
use std::fs;
use std::ops::{Index, IndexMut};
use std::path::PathBuf;

use eyre::Result;
use ractor::Actor;
use serde::{Deserialize, Serialize};
use tabled::builder::Builder;
use tabled::settings::Style;

use fetiche_common::{ConfigFile, IntoConfig, Versioned};

use crate::Sources;
use crate::{Auth, Site, SOURCES_CONFIG};

impl Sources {
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
        let fname: PathBuf = dir.join(SOURCES_CONFIG);
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
                    Auth::Vhost { .. } => "Virtual Host+login",
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
            let cap = s.feature.to_string();
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

/// Initialise a `Source` from a `BTreeMap`
///
impl From<BTreeMap<String, Site>> for Sources {
    fn from(value: BTreeMap<String, Site>) -> Self {
        Sources {
            site: value.clone(),
        }
    }
}

/// Initialise a `Source` from a `Vec` of (name, site)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::env::temp_dir;

    use crate::{Capability, DataType};
    use eyre::bail;
    use fetiche_common::ConfigFile;
    use tracing::debug;

    use super::*;

    #[test]
    fn test_sources_basic_operations() {
        let mut sources = Sources {
            site: BTreeMap::new(),
        };

        // Test empty state
        assert!(sources.is_empty());
        assert_eq!(sources.len(), 0);

        // Add a new site
        let site_name = "test_site";
        let site = Site::new();
        sources.site.insert(site_name.to_string(), site);

        // Test state after adding
        assert!(!sources.is_empty());
        assert_eq!(sources.len(), 1);

        // Test contains_key
        assert!(sources.contains_key(site_name));

        // Test get
        if let Some(retrieved_site) = sources.get(site_name) {
            assert_eq!(retrieved_site.base_url, "");
        } else {
            panic!("Site should exist");
        }

        // Test get_mut and modify
        if let Some(retrieved_site) = sources.get_mut(site_name) {
            retrieved_site.base_url = "http://example.com".to_string();
        }
        assert_eq!(
            sources.get(site_name).unwrap().base_url,
            "http://example.com"
        );

        // Test keys, values, and iter
        let keys: Vec<_> = sources.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, vec![site_name]);

        let values: Vec<_> = sources.values().collect();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].base_url, "http://example.com");

        let iter: Vec<_> = sources.iter().collect();
        assert_eq!(iter.len(), 1);
        assert_eq!(iter[0].0.as_str(), site_name);
        assert_eq!(iter[0].1.base_url, "http://example.com");
    }

    #[test]
    fn test_sources_mut_operations() {
        let mut sources = Sources {
            site: BTreeMap::new(),
        };

        let site_name1 = "site1";
        let site_name2 = "site2";

        // Use index_mut to add sites
        sources[site_name1].base_url = "http://site1.com".to_string();
        sources[site_name2].base_url = "http://site2.com".to_string();

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[site_name1].base_url, "http://site1.com");
        assert_eq!(sources[site_name2].base_url, "http://site2.com");

        // Modify site through index_mut
        sources[site_name1].base_url = "http://updated-site1.com".to_string();
        assert_eq!(sources[site_name1].base_url, "http://updated-site1.com");
    }

    #[test]
    fn test_sources_into_iter() {
        let mut sources = Sources {
            site: BTreeMap::new(),
        };

        sources.site.insert(
            "site1".to_string(),
            Site {
                feature: Capability::Fetch,
                base_url: "http://site1.com".to_string(),
                dtype: DataType::Drone,
                name: "".to_string(),
                token_base: Default::default(),
                auth: None,
                format: "".to_string(),
                routes: None,
                variant: None,
            },
        );
        sources.site.insert(
            "site2".to_string(),
            Site {
                feature: Capability::Fetch,
                base_url: "http://site2.com".to_string(),
                dtype: DataType::Adsb,
                name: "".to_string(),
                token_base: Default::default(),
                auth: None,
                format: "".to_string(),
                routes: None,
                variant: None,
            },
        );

        let iter: Vec<(&String, &Site)> = (&sources).into_iter().collect();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter[0].0, "site1");
        assert_eq!(iter[0].1.base_url, "http://site1.com");
        assert_eq!(iter[1].0, "site2");
        assert_eq!(iter[1].1.base_url, "http://site2.com");
    }

    #[test]
    fn test_sites_load_hcl() {
        let cn = PathBuf::from("src").join("sources").join("sources.hcl");
        assert!(cn.try_exists().is_ok());

        let cfile = ConfigFile::<SourcesConfig>::load(Some(&cn.to_string_lossy().to_string()));
        assert!(cfile.is_ok());

        let cfile = cfile.unwrap();
        let cfg = cfile.inner();
        assert!(!cfg.site.is_empty());
        assert_eq!(9, cfg.site.len());

        // Check one
        //
        if let Some(site) = cfg.site.get("eih") {
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
        if let Some(site) = cfg.site.get("opensky") {
            assert_eq!("https://opensky-network.org/api", site.base_url);
            assert_eq!(DataType::Adsb, site.dtype);
            match &site.auth {
                Some(auth) => match auth {
                    Auth::Login {
                        username, password, ..
                    } => {
                        assert_eq!("GUESS", username);
                        assert_eq!("NEVER", password);
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
                let f = tempdir.join(SOURCES_CONFIG);
                assert!(f.exists());
            }
            _ => bail!("all failed"),
        }
        Ok(())
    }
}
