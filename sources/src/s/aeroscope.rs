//! Aeroscope site-specifics
//!
//! Phases:
//! 1. use the configured login & password to obtain a token
//! 2. use the token to get the data
//!
//! Data fetched is json and not csv but our struct in `format-specs/aeroscope.rs`  is compatible with
//! both, even flattening the different lat/long structs in a sensible way.
//!
//! This implement the `Fetchable` trait described in `site/lib`.
//!

use anyhow::Result;
use clap::{crate_name, crate_version};
use log::{debug, error, trace};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use format_specs::aeroscope::Aeroscope as InputFormat;
use format_specs::Cat21;
use format_specs::Format;

use crate::site::{Auth, Site};
use crate::{http_get_auth, http_post, Fetchable};

/// Data to send to authenticate ourselves and get a token
///
#[derive(Debug, Serialize)]
struct Credentials {
    /// Username
    username: String,
    /// Password
    password: String,
}

/// Access token derived from username/password
///
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Token {
    /// Token (SHA-256 or -512 data I guess)
    access_token: String,
}

/// This describe the Aeroscope "site" which is the PC we have here at the EIH
/// ///
#[derive(Clone, Debug)]
pub struct Aeroscope {
    /// Input format-specs
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

    /// Load our site details from what is in the configuration file
    ///
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("Loading {site:?}");
        self.format = site.format.as_str().into();
        self.base_url = site.base_url.to_owned();
        if let Some(auth) = &site.auth {
            match auth {
                Auth::Token {
                    token,
                    login,
                    password,
                } => {
                    self.token = token.to_owned();
                    self.login = login.to_owned();
                    self.password = password.to_owned();
                }
                _ => panic!("nope"),
            }
        }
        self.get = site.cmd.get.to_owned();
        self
    }
}

impl Default for Aeroscope {
    fn default() -> Self {
        Self::new()
    }
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

        let resp = http_post!(self, url, &cred)?;
        let resp = resp.text()?;
        let res: Token = serde_json::from_str(&resp)?;

        debug!("res={:?}", res);
        Ok(res.access_token)
    }

    /// Fetch actual data from the site as a long String.
    ///
    fn fetch(&self, token: &str, _args: &str) -> Result<String> {
        debug!("Now fetching data");

        // Use the token to authenticate ourselves
        //
        let url = format!("{}{}", self.base_url, self.get);
        let resp = http_get_auth!(self, url, token);
        let resp = resp?.text()?;

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
            // Skip if element doesn't have any position
            .filter(|line| line.coordinate.latitude != 0.0 && line.coordinate.longitude != 0.0)
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
            .collect();
        debug!("res={:?}", res);
        Ok(res)
    }

    /// Returns the site's input format-specs
    ///
    fn format(&self) -> Format {
        Format::Aeroscope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::aeroscope::Aeroscope as InputFormat;

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

    // #[test]
    // fn test_get_aeroscope_data() {
    //     let server = MockServer::start();
    //     let token = "FOOBAR".to_string();
    //
    //     let data = InputFormat;
    //     let jtok = json!(data);
    //     let m = server.mock(|when, then| {
    //         when.method(POST)
    //             .header(
    //                 "user-agent",
    //                 format!("{}/{}", crate_name!(), crate_version!()),
    //             )
    //             .header("content-type", "application/json")
    //             .bearer_auth(token)
    //             .path("/get");
    //         then.status(200).body(&jtok);
    //     });
    //
    //     let client = Client::new();
    //     let site = Aeroscope {
    //         format: Format::Aeroscope,
    //         login: "user".to_string(),
    //         password: "pass".to_string(),
    //         token: "/login".to_string(),
    //         base_url: server.base_url().clone(),
    //         get: "/get".to_string(),
    //         client,
    //     };
    //     let data = site.fetch(&token);
    //
    //     m.assert();
    //     assert!(data.is_ok());
    //     let data = data.unwrap();
    // }
}
