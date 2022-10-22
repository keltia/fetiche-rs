//! Module to deal with different kind of input be it a file or a site with a, API
//!
//! When no file is specified on the command-line, we look at the list of possible sites to fetch
//! data from, each with a known format.  We also define here the URL and associated credentials
//! (if any) needed.
//!
//! If the `token` URL is present, we call this first with `POST`  to request an OAuth2 token.  
//! We assume the output format to be the same with `{ access_token: String }`.
//!

use anyhow::{anyhow, bail, Result};
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};

use std::fs;
use std::path::PathBuf;

use crate::format::Source;
use crate::Config;

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
        /// Token fetching URL (if present call this first)
        token: String,
        /// Data fetching URL
        get: String,
        /// Login (if needed)
        login: String,
        /// Password if needed
        password: String,
    },
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

/// Access token derived from username/password
///
#[derive(Debug, Deserialize)]
struct Token {
    /// Token (SHA-256 or -512 data I guess)
    access_token: String,
}

impl Site {
    /// Initialize a site by checking whether it is present in the configuration file
    ///
    pub fn new(cfg: &Config, site: &str) -> Self {
        if cfg.sites.contains_key(site) {
            return cfg.sites[site].clone();
        }
        error!("{site} not found!");
        Site::Invalid
    }

    pub fn format(&self) -> Source {
        match self {
            Site::Login { format, .. } | Site::Anon { format, .. } => Source::from_str(format),
            _ => Source::None,
        }
    }

    /// Fetch the access token linked to the given login/password
    ///
    fn fetch_token(&self, client: &reqwest::blocking::Client) -> Result<String> {
        match self {
            Site::Login {
                base_url,
                login,
                password,
                token,
                ..
            } => {
                // Prepare our data
                //
                let body = format!(
                    "{{\"username\": \"{}\", \"password\": \"{}\"}}",
                    login, password
                );

                // fetch token
                //
                let url = format!("{}{}", base_url, token);
                trace!("Fetching token through {}…", url);
                let resp = client
                    .post(url)
                    .header("content-type", "application/json")
                    .body(body)
                    .send();

                let resp = resp?.text()?;

                let res: Token = serde_json::from_str(&resp)?;
                debug!("{:?}", res);
                Ok(res.access_token)
            }
            _ => return Err(anyhow!("no credential needed")),
        }
    }

    /// Using the access token obtained through `fetch_token()`, fetch the given CSV data
    ///
    pub fn fetch(&self) -> Result<String> {
        info!("Fetch data from network…");

        let client = reqwest::blocking::Client::new();
        let resp = match self {
            Site::Login { base_url, get, .. } => {
                // First call to gen auth token
                //
                let token = &self.fetch_token(&client)?;

                // Use the token to authenticate ourselves
                //
                let url = format!("{}{}", base_url, get);
                client
                    .get(url)
                    .header("content-type", "application/json")
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
            }
            Site::Anon { base_url, get, .. } => {
                let url = format!("{}{}", base_url, get);
                client
                    .get(url)
                    .header("content-type", "application/json")
                    .send()
            }
            Site::Invalid => panic!("nope"),
        };

        match resp {
            Ok(resp) => Ok(resp.text().unwrap()),
            Err(e) => bail!("HTTP error: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::Config;
    use std::path::PathBuf;
    use toml::to_string;

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
    fn test_site_new() {
        let cfg = set_default();
        dbg!(toml::to_string(&cfg).unwrap());

        let s = Site::new(&cfg, "foo");
        match s {
            Site::Anon {
                format,
                base_url,
                get,
            } => {
                assert_eq!("aeroscope", format);
                assert_eq!("http://example.net/", &base_url);
                assert_eq!("/get", &get);
            }
            _ => (),
        }
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
