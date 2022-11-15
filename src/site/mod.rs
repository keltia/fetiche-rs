//! Module to deal with different kind of sites we can connect to to fetch data.
//!
//! The different submodules deal with the differences between sites:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!

pub mod aeroscope;
pub mod asd;
pub mod safesky;

use anyhow::{anyhow, Result};
use log::trace;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::config::Config;
use crate::format::{Cat21, Format};
use crate::site::aeroscope::Aeroscope;
use crate::site::asd::Asd;
use crate::site::safesky::Safesky;

/// This trait enables us to manage different ways of connecting and fetching data under
/// a single interface.
///
pub trait Fetchable: Debug {
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String>;
    /// Fetch actual data
    fn fetch(&self, token: &str, args: &str) -> Result<String>;
    /// Transform fetched data into Cat21
    fn process(&self, input: String) -> Result<Vec<Cat21>>;
    /// Returns the input format
    fn format(&self) -> Format;
}

/// This macro refactor the code to call the HTTP client
///
#[macro_export]
macro_rules! http_call {
    /// Get token by submitted credentials
    ///
    ($self:ident, $url:ident, $cred:expr) => {
        $self
            .client
            .clone()
            .post($url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .json($cred)
            .send()
    };
    /// Auth'd call with data submission
    ///
    ($self:ident, $url:ident, $token:ident, $data:expr) => {
        $self
            .client
            .clone()
            .post($url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .bearer_auth($token)
            .json($data)
            .send()
    };
    /// Auth'd call without any data
    ///
    ($self:ident, $url:ident, $token:ident) => {
        $self
            .client
            .clone()
            .post($url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .bearer_auth($token)
            .send()
    };
}

/// Describe what a site is and associated credentials.
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Site {
    /// Site that needs to be supplied a username and password, often to get a token
    ///
    Login {
        /// Type of input
        format: String,
        /// Base URL (to avoid repeating)
        base_url: String,
        /// Token fetching URL (if present call this first)
        token: String,
        /// Data fetching URL
        get: String,
        /// Login (if needed)
        login: String,
        /// Password if needed
        password: String,
    },
    /// Site using an API key, supplied in a header or in the URL
    ///
    Key {
        /// Type of input
        format: String,
        /// Base URL
        base_url: String,
        /// API key as x-api-key:
        api_key: String,
        /// Data fetching URL
        get: String,
    },
    /// Plain anonymous public access
    ///
    Anon {
        /// Type of input
        format: String,
        /// Base URL (to avoid repeating)
        base_url: String,
        /// Data fetching URL
        get: String,
    },
    Invalid,
}

impl Site {
    /// Load site by checking whether it is present in the configuration file
    ///
    pub fn load(name: &str, cfg: &Config) -> Result<Box<dyn Fetchable>> {
        trace!("Loading site {}", name);
        match cfg.sites.get(name) {
            Some(site) => {
                let fmt = site.format();
                match fmt {
                    Format::Aeroscope => {
                        let s = Aeroscope::new().load(site).clone();

                        Ok(Box::new(s))
                    }
                    Format::Asd => {
                        let s = Asd::new().load(site).clone();

                        Ok(Box::new(s))
                    }
                    Format::Safesky => {
                        let s = Safesky::new().load(site).clone();

                        Ok(Box::new(s))
                    }
                    _ => Err(anyhow!("invalid site {name}")),
                }
            }
            None => Err(anyhow!("no such site {name}")),
        }
    }

    /// Return the site format
    ///
    pub fn format(&self) -> Format {
        match self {
            Site::Login { format, .. } | Site::Key { format, .. } | Site::Anon { format, .. } => {
                format.as_str().into()
            }
            _ => Format::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::makepath;

    fn set_default() -> Config {
        let s = Site::Anon {
            format: "aeroscope".to_string(),
            base_url: "http://example.net/".to_string(),
            get: "/get".to_string(),
        };

        let mut h: HashMap<String, Site> = HashMap::new();
        h.insert("foo".to_string(), s);

        let cfg = Config {
            default: "none".to_string(),
            sites: h,
        };

        cfg
    }

    #[test]
    fn test_site_new_good() {
        let cfg = set_default();

        let s = Site::load("foo", &cfg);
        assert!(s.is_ok());
    }

    #[test]
    fn test_site_new_unknown() {
        let cfg = set_default();

        let s = Site::load("bar", &cfg);
        assert!(s.is_err());
    }

    #[test]
    fn test_site_loading() {
        let cfn: PathBuf = makepath!("src", "bin", "cat21conv", "config.toml");
        let cfg = Config::load(&cfn);
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        let s = cfg.sites.clone();

        assert_eq!("none", cfg.default);
        assert!(!s.is_empty());
        assert_eq!(4, s.len());
        assert!(s.contains_key("nope"));

        for (_, s) in s.iter() {
            match s {
                Site::Anon { format, .. } => {
                    assert_eq!("aeroscope", format);
                }
                Site::Key { format, .. } => {
                    assert_eq!("safesky", format);
                }
                Site::Login {
                    format,
                    base_url,
                    token,
                    ..
                } => {
                    assert_ne!("none", format);
                    assert!(!base_url.is_empty());
                    assert!(!token.is_empty());
                }
                Site::Invalid => panic!("nope"),
            }
        }
    }
}
