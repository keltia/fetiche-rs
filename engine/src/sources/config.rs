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
use serde::{Deserialize, Serialize};
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::trace;

#[cfg(feature = "aeroscope")]
use crate::Aeroscope;
#[cfg(feature = "asd")]
use crate::Asd;
#[cfg(feature = "avionix")]
use crate::AvionixServer;
#[cfg(feature = "avionix")]
use crate::Cube;
#[cfg(feature = "flightaware")]
use crate::Flightaware;
#[cfg(feature = "opensky")]
use crate::Opensky;
#[cfg(feature = "safesky")]
use crate::Safesky;
#[cfg(feature = "senhive")]
use crate::Senhive;
use crate::{AccessError, Auth, FetchableSource, Site, StreamableSource, SOURCES_CONFIG};

use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_formats::Format;
use fetiche_macros::into_configfile;

/// Configuration for multiple sources.
///
/// This struct holds a configuration of sites, which are represented
/// as a mapping between their string identifiers and their corresponding
/// [`Site`] configurations.
///
/// It can be initialized from various data types, such as a [`BTreeMap`]
/// or a `Vec` of tuples using the provided `From` implementations.
///
/// # Examples
///
/// Creating `SourcesConfig` from a `BTreeMap`:
///
/// ```rust
/// use std::collections::BTreeMap;
/// use fetiche_engine::{Site, Sources};
///
/// let mut sites = BTreeMap::new();
/// sites.insert("example_site".to_string(), Site::default());
///
/// let sources = Sources::from(sites);
/// assert!(sources.contains_key("example_site"));
/// ```
///
/// Creating `SourcesConfig` from a vector of tuples:
///
/// ```rust
/// use fetiche_engine::{Site, Sources};
///
/// let sites_vec = vec![("site_a".to_string(), Site::default())];
/// let sources = Sources::from(sites_vec);
/// assert!(sources.contains_key("site_a"));
/// ```
///
#[into_configfile(version = 4, filename = "sources.hcl")]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SourcesConfig {
    site: BTreeMap<String, Site>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sources {
    site: BTreeMap<String, Site>,
}

impl Sources {
    /// Creates a new instance of `Sources` by loading the configuration from
    /// the predefined `sources.hcl` configuration file.
    ///
    /// This function attempts to parse the configuration file as a
    /// `ConfigFile<SourcesConfig>` and initializes the `Sources` instance
    /// from the parsed configuration. The resulting data is transformed
    /// to attach additional metadata, such as the root directory as
    /// `token_base`, to each site.
    ///
    /// # Errors
    ///
    /// Returns an `Err` variant if the `sources.hcl` configuration file
    /// cannot be found, fails to parse, or if there are any issues when
    /// constructing the `Sources` object from the configuration.
    ///
    #[tracing::instrument]
    pub fn new() -> Result<Self> {
        let src_file = ConfigFile::<SourcesConfig>::load(Some(SOURCES_CONFIG))?;
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

    pub fn as_fetchable(&self, name: &str) -> Result<FetchableSource> {
        match self.site.get(name) {
            Some(site) => {
                trace!("site={}", site);
                let fmt = site.format();

                // We have to explicitly list all supported formats as we return
                // an enum whether the site will be fetchable or not
                //
                match fmt {
                    #[cfg(feature = "asd")]
                    Format::Asd => {
                        let s = Asd::new().load(site).clone();
                        Ok(FetchableSource::Asd(s))
                    }
                    #[cfg(feature = "aeroscope")]
                    Format::Aeroscope => {
                        let s = Aeroscope::new().load(site).clone();
                        Ok(FetchableSource::Aeroscope(s))
                    }
                    #[cfg(feature = "safesky")]
                    Format::Safesky => {
                        let s = Safesky::new().load(site).clone();
                        Ok(FetchableSource::Safesky(s))
                    }
                    _ => Err(AccessError::InvalidSite(name.to_string()).into()),
                }
            }
            None => Err(AccessError::UnknownSite(name.to_string()).into()),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn as_streamable(&self, name: &str) -> Result<StreamableSource> {
        match self.site.get(name) {
            Some(site) => {
                trace!("site={}", site);
                let fmt = site.format();

                // We have to explicitly list all supported formats as we return
                // an enum whether the site will be streamable or not
                //
                match fmt {
                    #[cfg(feature = "avionix")]
                    Format::CubeData => {
                        if let Some(Auth::UserKey { .. }) = site.auth {
                            let s = AvionixServer::new().load(site).clone();
                            Ok(StreamableSource::AvionixServer(s))
                        } else {
                            let s = Cube::new().load(site).clone();
                            Ok(StreamableSource::Cube(s))
                        }
                    }
                    #[cfg(feature = "opensky")]
                    Format::Opensky => {
                        let s = Opensky::new().load(site).clone();

                        Ok(StreamableSource::Opensky(s))
                    }
                    #[cfg(feature = "flightaware")]
                    Format::Flightaware => {
                        let s = Flightaware::new().load(site).clone();

                        Ok(StreamableSource::Flightaware(s))
                    }
                    #[cfg(feature = "senhive")]
                    Format::Senhive => {
                        let s = Senhive::new().load(site).clone();

                        Ok(StreamableSource::Senhive(s))
                    }
                    _ => Err(AccessError::InvalidSite(name.to_string()).into()),
                }
            }
            None => Err(AccessError::UnknownSite(name.to_string()).into()),
        }
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
        let cn = PathBuf::from("src").join("sources.hcl");
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
