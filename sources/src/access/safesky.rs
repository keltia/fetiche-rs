//! Safesky specifics
//!
//! Phase:
//! 1. obtain an API key from a mail sent to them
//! 2. use x-api-key: KEY for all submissions to public-api.safesky.app/beacons
//!
//! Format is take from the CSV given as an example
//!

use std::io::Write;

use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use fetiche_formats::{Format, Position};

use crate::site::Site;
use crate::{Auth, Capability, Fetchable};

/// Define the square inside which we want beacons information
///
#[derive(Debug, Deserialize, Serialize)]
struct Viewport {
    /// North-West corner
    nw: Position,
    /// South-East corner
    se: Position,
}

/// Data to submit to get replay of journeys
///
#[derive(Debug, Serialize)]
struct Param {
    /// Limit ourselves to this time interval beginning at
    altitude_min: Option<i32>,
    /// Limit ourselves to this time interval ending at
    altitude_max: Option<i32>,
    /// Beacon types e.g. "UNKNOWN,GLIDER,PARA_GLIDER"
    beacon_type: Option<String>,
    /// Also show grounded beacons?
    show_grounded: bool,
    /// Mandatory:
    viewport: Viewport,
}

#[derive(Clone, Debug)]
pub struct Safesky {
    /// Describe the different features of the source
    pub features: Vec<Capability>,
    /// Format of data
    pub format: Format,
    /// Base URL for the API
    pub base_url: String,
    /// Route to get data
    pub get: String,
    /// Safesky uses an API
    pub api_key: String,
    /// HTTP Client
    pub client: Client,
}

impl Safesky {
    pub fn new() -> Self {
        Safesky {
            features: vec![Capability::Fetch],
            format: Format::Safesky,
            base_url: "".to_owned(),
            api_key: "".to_owned(),
            get: "".to_owned(),
            client: Client::new(),
        }
    }

    pub fn load(&mut self, site: &Site) -> &mut Self {
        self.format = site.format.as_str().into();
        self.base_url = site.base_url.to_owned();
        if let Some(auth) = &site.auth {
            match auth {
                Auth::Key { api_key } => {
                    self.api_key = api_key.to_owned();
                }
                _ => panic!("nope"),
            }
        }
        self.get = site.route("get").unwrap().to_owned();
        self
    }
}

impl Default for Safesky {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Safesky {
    fn name(&self) -> String {
        "safesky".to_string()
    }
    /// Safesky is using an API key you need to have for all transactions, there is no
    /// real authentication.
    ///
    fn authenticate(&self) -> Result<String> {
        if self.api_key.is_empty() {
            return Err(anyhow!("No API key"));
        }
        Ok(self.api_key.clone())
    }

    fn fetch(&self, _out: &mut dyn Write, _token: &str, _args: &str) -> Result<()> {
        todo!()
    }

    fn format(&self) -> Format {
        Format::Safesky
    }
}

#[cfg(test)]
mod tests {
    use clap::{crate_name, crate_version};
    use httpmock::Method::GET;
    use httpmock::MockServer;

    use super::*;

    fn setup_safesky(_server: &MockServer) -> Safesky {
        let client = Client::new();
        Safesky {
            features: Capability::Fetch,
            format: Format::Safesky,
            base_url: "http://example.net".to_string(),
            get: "/v1/beacons".to_string(),
            api_key: "FOOBAR".to_string(),
            client: client.clone(),
        }
    }

    #[test]
    fn test_safesky_load() {
        let server = MockServer::start();
        let _m = server.mock(|when, then| {
            when.method(GET)
                .header(
                    "user-agent",
                    format!("{}/{}", crate_name!(), crate_version!()),
                )
                .header("content-type", "application/json")
                .path("/api/security/login");
            then.status(200);
        });

        let _site = setup_safesky(&server);
    }
}
