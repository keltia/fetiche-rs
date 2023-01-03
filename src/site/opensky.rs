//! OpenSky (.org) specific data
//!

use log::error;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::format::{Cat21, Format};
use crate::site::{Fetchable, Site};

#[derive(Clone, Debug)]
pub struct Opensky {
    /// Input format
    pub format: Format,
    /// Username
    pub login: String,
    /// Password
    pub password: String,
    /// Base site url taken from config
    pub base_url: String,
    /// Add this to `base_url` to fetch data
    pub get: String,
    /// reqwest blocking client
    pub client: Client,
}

#[derive(Debug, Serialize)]
struct Param {
    /// timestamp of the state vectors to be retrieved
    pub time: Option<u32>,
    /// One or more ICAO24 transponder address
    pub icao24: Option<Vec<String>>,
    /// One or more receiver IDs
    pub serials: Option<Vec<u32>>,
}

/// Credentials to submit to the site to get the token
///
#[derive(Debug, Serialize)]
struct Credentials {
    /// Email as username
    username: String,
    /// Password
    password: String,
}

impl Opensky {
    pub fn new() -> Self {
        Opensky {
            format: Format::None,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            client: Client::new(),
        }
    }

    /// Load some data from the configuration file
    ///
    pub fn load(&mut self, site: &Site) -> &mut Self {
        match site {
            Site::Login {
                format,
                base_url,
                login,
                password,
                get,
                ..
            } => {
                self.format = format.as_str().into();
                self.base_url = base_url.to_owned();
                self.get = get.to_owned();
                self.login = login.to_owned();
                self.password = password.to_owned();
            }
            _ => {
                error!("Missing config data for {site:?}")
            }
        }
        self
    }
}

impl Default for Opensky {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Opensky {
    fn authenticate(&self) -> anyhow::Result<String> {
        todo!()
    }

    fn fetch(&self, token: &str, args: &str) -> anyhow::Result<String> {
        todo!()
    }

    fn format(&self) -> Format {
        todo!()
    }

    fn process(&self, input: String) -> anyhow::Result<Vec<Cat21>> {
        todo!()
    }
}
