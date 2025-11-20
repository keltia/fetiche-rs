//! This module contains the `Sources` actor support code, which manages a collection of
//! [`Site`] configurations.
//!
//! The `Sources` actor is responsible for loading and managing the configuration
//! of multiple [`Site`]s, which are represented as a mapping between their
//! string identifiers and their corresponding [`Site`] configurations.
//!
//! The `Sources` actor is initialized by loading the configuration from the
//! predefined `sources.hcl` configuration file. The configuration file is
//! parsed as a [`ConfigFile<SourcesConfig>`] as part of the actor init code.
//!
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use eyre::{eyre, Result};
use ractor::{call, Actor, ActorRef};
use serde::{Deserialize, Serialize};
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::{info, trace};

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
#[cfg(feature = "safesky")]
use crate::Safesky;
#[cfg(feature = "senhive")]
use crate::Senhive;
#[cfg(feature = "opensky")]
use crate::{OpenskyDevice, OpenskyServer};

use crate::{
    AccessError, Auth, FetchableSource, Site, SourcesActor, SourcesMsg, StreamableSource,
    SOURCES_CONFIG,
};

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
    pub(crate) site: BTreeMap<String, Site>,
}

impl Default for Sources {
    fn default() -> Self {
        Self {
            site: BTreeMap::new(),
        }
    }
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
    pub fn new() -> eyre::Result<Self> {
        let src_file = ConfigFile::<SourcesConfig>::load(Some(SOURCES_CONFIG))?;
        let src = src_file.inner();

        let all = src
            .site
            .iter()
            .map(|(n, s)| {
                let mut site = s.clone();

                site.name = n.to_string();
                site.token_base = Some(src_file.root().join(site.token_base.unwrap_or("".into())));
                (n.to_string(), site)
            })
            .collect::<Vec<_>>();
        let s = crate::Sources::from(all);
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
    pub fn as_streamable(&self, name: &str) -> eyre::Result<StreamableSource> {
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
                        if let Some(Auth::Login { .. }) = site.auth {
                            let s = OpenskyServer::new().load(site).clone();
                            Ok(StreamableSource::OpenskyServer(s))
                        } else {
                            let s = OpenskyDevice::new().load(site).clone();
                            Ok(StreamableSource::OpenskyDevice(s))
                        }
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
}
