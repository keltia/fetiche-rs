//! Aeroscope site-specifics
//!
//! Phases:
//! 1. use the configured login & password to obtain a token
//! 2. use the token to get the data
//!
//! Data fetched is json and not csv but our struct in `format/aeroscope.rs`  is compatible with
//! both, even flattening the different lat/long structs in a sensible way.
//!
//! This implement the `Fetchable` trait described in `site/mod.rs`.
//!

use anyhow::Result;
use clap::{crate_name, crate_version};
use log::{debug, error};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::format::aeroscope::Aeroscope as InputFormat;
use crate::format::{Cat21, Format};
use crate::site::{Fetchable, Site};

/// This describe the Aeroscope "site" which is the PC we have here at the EIH
/// ///
#[derive(Clone, Debug)]
pub struct Aeroscope {
    /// Input format
    pub format: Format,
    /// Auth data, username
    pub login: String,
    /// Auth data, password
    pub password: String,
    /// Base site url taken from config
    pub base_url: String,
    /// Add this to `base_url` for token
    pub token: String,
    /// Add this to `base_url` to fetch data
    pub get: String,
    /// reqwest clocking client
    pub client: Client,
}

impl Aeroscope {
    pub fn new() -> Self {
        // Set some reasonable defaults
        //
        Aeroscope {
            format: Format::None,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            token: "".to_owned(),
            client: Client::new(),
        }
    }

    /// Load our site details from what is in the confifguration file
    ///
    pub fn load(&mut self, site: &Site) -> &mut Self {
        match site {
            Site::Login {
                format,
                base_url,
                login,
                password,
                token,
                get,
                ..
            } => {
                self.format = format.as_str().into();
                self.base_url = base_url.to_owned();
                self.get = get.to_owned();
                self.token = token.to_owned();
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

impl Default for Aeroscope {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct Credentials {
    username: String,
    password: String,
}

impl Fetchable for Aeroscope {
    /// Authenticate to the site with login/password and return a token
    ///
    fn authenticate(&self) -> Result<String> {
        // Prepare our submission data
        //
        debug!("Submit auth as {:?}", &self.login);
        let cred = Credentials {
            username: self.login.clone(),
            password: self.password.clone(),
        };

        // fetch token
        //
        let url = format!("{}{}", self.base_url, self.token);
        debug!("Fetching token through {}…", url);
        let resp = self
            .client
            .clone()
            .post(url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .json(&cred)
            .send();

        let resp = resp?.text()?;
        let res: Token = serde_json::from_str(&resp)?;
        debug!("{:?}", res);
        Ok(res.access_token)
    }

    /// Fetch actual data from the site as a long String.
    ///
    fn fetch(&self, token: &str, _args: &str) -> Result<String> {
        debug!("Now fetching data");
        // Use the token to authenticate ourselves
        //
        let url = format!("{}{}", self.base_url, self.get);
        let resp = self
            .client
            .clone()
            .get(url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .bearer_auth(token)
            .send()?
            .text()?;
        debug!("{} bytes read. ", resp.len());
        Ok(resp)
    }

    /// Process data fetch in previous stage and render it as wanted
    ///
    fn process(&self, input: String) -> Result<Vec<Cat21>> {
        debug!("Reading & transforming…");
        debug!("IN={:?}", input);
        let res: Vec<InputFormat> = serde_json::from_str(&input)?;

        let res = res
            .iter()
            // Add "line number" for output
            .enumerate()
            // Debug
            .inspect(|(n, f)| println!("res={:?}-{:?}", n, f))
            // Convert
            .map(|(cnt, rec)| {
                debug!("rec={:?}", rec);
                let mut line = Cat21::from(rec);
                line.rec_num = cnt;
                line
            })
            // Skip if element doesn't have any position
            .filter(|line| line.pos_lat_deg != 0.0 && line.pos_long_deg != 0.0)
            .collect();
        debug!("res={:?}", res);
        Ok(res)
    }

    fn format(&self) -> Format {
        Format::Aeroscope
    }
}

/// Access token derived from username/password
///
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Token {
    /// Token (SHA-256 or -512 data I guess)
    access_token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn test_get_aeroscope_token() {
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

        let client = Client::new();
        let site = Aeroscope {
            format: Format::Aeroscope,
            login: "user".to_string(),
            password: "pass".to_string(),
            token: "/login".to_string(),
            base_url: server.base_url().clone(),
            get: "/get".to_string(),
            client,
        };
        let t = site.authenticate();

        m.assert();
        assert!(t.is_ok());
        assert_eq!("FOOBAR", t.as_ref().unwrap());
    }
}
