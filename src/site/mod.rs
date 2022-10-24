//! Module to deal with different kind of input be it a file or a site with a, API
//!
//! When no file is specified on the command-line, we look at the list of possible sites to fetch
//! data from, each with a known format.  We also define here the URL and associated credentials
//! (if any) needed.
//!
//! If the `token` URL is present, we call this first with `POST`  to request an OAuth2 token.  
//! We assume the output format to be the same with `{ access_token: String }`.
//!

pub mod aeroscope;
pub mod asd;
pub mod auth;
pub mod safesky;

use anyhow::{anyhow, bail, format_err, Result};
use clap::{crate_name, crate_version};
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};

use crate::format::Source;
use crate::site::aeroscope::Aeroscope;
use crate::site::asd::Asd;
use crate::site::auth::Auth;
use crate::site::safesky::Safesky;
use crate::Config;

pub trait Fetchable {
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String>;
    /// Fetch actual data
    fn fetch(&self, token: &str) -> Result<String>;
    /// Setup pre-fetch stuff
    fn prefetch(&self, token: &str) -> Result<String>;
    /// Returns the input format
    fn format(&self) -> Source {
        self.format
    }
}

/// Describe what a site is and associated credentials.
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Site {
    Login {
        /// Type of input
        format: String,
        /// Base URL (to avoid repeating)
        base_url: String,
        /// Auth submit format
        auth: String,
        /// Token fetching URL (if present call this first)
        token: String,
        /// Data fetching URL
        get: String,
        /// Login (if needed)
        login: String,
        /// Password if needed
        password: String,
        /// If a prefetch action is needed, prefetch should be true
        prefetch: bool,
    },
    Key {
        /// Type of input
        format: String,
        /// Base URL
        base_url: String,
        /// API key as x-api-key:
        api_key: String,
        /// Data fetching URL
        get: String,
        /// If a prefetch action is needed, prefetch should be true
        prefetch: bool,
    },
    Anon {
        /// Type of input
        format: String,
        /// Base URL (to avoid repeating)
        base_url: String,
        /// Data fetching URL
        get: String,
        /// If a prefetch action is needed, prefetch should be true
        prefetch: bool,
    },
    Invalid,
}

impl Site {
    /// Initialize a site by checking whether it is present in the configuration file
    ///
    pub fn new(cfg: &Config, site: &str) -> Result<Box<dyn Fetchable>> {
        let s = match cfg.sites.contains_key(site) {
            None => Err(format_err!("{site} not found!")),
            Some(site) => match site {
                "aeroscope" => Box::new(Aeroscope::new().load(&cfg)),
                "asd" => Box::new(Asd::new().load(&cfg)),
                "safesky" => Box::new(Safesky::new().load(&cfg)),
            },
        }?;
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::Config;
    use clap::{crate_name, crate_version};
    use std::path::PathBuf;

    fn set_default() -> Config {
        let s = Site::Anon {
            format: "aeroscope".to_string(),
            base_url: "http://example.net/".to_string(),
            get: "/get".to_string(),
            prefetch: false,
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
        dbg!(toml::to_string(&cfg).unwrap());

        let s = Site::new(&cfg, "foo");
        assert!(s.is_ok());
    }

    #[test]
    fn test_site_new_unknown() {
        let cfg = set_default();
        dbg!(toml::to_string(&cfg).unwrap());

        let s = Site::new(&cfg, "bar");
        assert!(s.is_err());
    }

    #[test]
    fn test_site_loading() {
        let cfn = PathBuf::from("src/config.toml");
        let cfg = Config::load(&cfn);
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        let s = cfg.sites;

        assert_eq!("none", cfg.default);
        assert!(!s.is_empty());
        assert_eq!(3, s.len());
        assert!(s.contains_key("nope"));

        for (_, s) in s.iter() {
            match s {
                Site::Anon { format, .. } => {
                    assert_eq!("safesky", format);
                }
                Site::Key { format, .. } => {
                    assert!("none", format);
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

    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn test_get_token() {
        let server = MockServer::start();
        let token = Token {
            access_token: "FOOBAR".to_string(),
        };
        let jtok = json!(token).to_string();
        let m = server.mock(|when, then| {
            when.method(POST)
                .header(
                    "user-agent",
                    format!("{}/{}", crate_name!(), crate_version!()),
                )
                .header("content-type", "application/json")
                .path("/login");
            then.status(200).body(&jtok);
        });

        let client = reqwest::blocking::Client::new();
        let site = Site::Login {
            format: "aeroscope".to_string(),
            login: "user".to_string(),
            auth: "aeroscope".to_string(),
            password: "pass".to_string(),
            token: "/login".to_string(),
            base_url: server.base_url().clone(),
            get: "/get".to_string(),
            prefetch: false,
        };
        let t = site.fetch_token(&client);

        m.assert();
        assert!(t.is_ok());
        assert_eq!("FOOBAR", t.as_ref().unwrap());
    }
}
