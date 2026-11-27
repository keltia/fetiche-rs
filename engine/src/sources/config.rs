//! HCL source configuration, default installation, and path resolution.
//!
//! The engine can load sources from an explicit configuration directory or the
//! standard configuration location used by `Sources::new`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use eyre::Result;
use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;
use serde::Deserialize;

use super::{SOURCES_CONFIG, Site, Sources};

/// Configuration for multiple sources.
///
/// This struct holds a configuration of sites, which are represented
/// as a mapping between their string identifiers and their corresponding
/// [`Site`] configurations.
///
#[into_configfile(version = 4, filename = "sources.hcl")]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SourcesConfig {
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
        Ok(Self::from_config(src_file.inner(), &src_file.root()))
    }

    /// Loads sources and resolves their token paths relative to the named configuration directory.
    pub(crate) fn load(home: &Path) -> Result<Self> {
        let file = home.join(SOURCES_CONFIG);
        let config: SourcesConfig = hcl::from_str(&fs::read_to_string(file)?)?;
        Ok(Self::from_config(&config, home))
    }

    /// Attaches source identifiers and token directories to parsed configuration entries.
    fn from_config(config: &SourcesConfig, home: &Path) -> Self {
        let all = config
            .site
            .iter()
            .map(|(n, s)| {
                let mut site = s.clone();

                site.name = n.to_string();
                site.token_base = Some(home.join(site.token_base.unwrap_or_default()));
                (n.to_string(), site)
            })
            .collect::<Vec<_>>();
        Sources::from(all)
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
}

#[cfg(test)]
mod tests {
    use crate::{Auth, DataType};

    use super::*;

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

    /// Installs the embedded HCL and checks that explicit loading adds site metadata.
    #[test]
    fn installs_and_loads_default_sources_from_explicit_home() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let home = temp.path().join("config");
        Sources::install_defaults(&home)?;
        assert_eq!(
            fs::read_to_string(home.join(SOURCES_CONFIG))?,
            include_str!("sources.hcl")
        );
        let sources = Sources::load(&home)?;
        let site = sources.get("eih").unwrap();
        assert_eq!(site.name, "eih");
        assert_eq!(site.token_base.as_deref(), Some(home.as_path()));
        Ok(())
    }
}
