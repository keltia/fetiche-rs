//! Aeroscope site-specifics
//!
//! Phases:
//! 1. use the configured login & password to obtain a token
//! 2. use the token to get the data
//!
//! Data fetched is json and not csv but our struct in `formats/aeroscope.rs`  is compatible with
//! both, even flattening the different lat/long structs in a sensible way.
//!
//! This implement the `Fetchable` trait described in `mod.rs`.
//!

use std::str::FromStr;

use chrono::Utc;
use clap::{crate_name, crate_version};
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::mpsc::Sender;
use tracing::{debug, trace};

use fetiche_formats::Format;

use crate::{Auth, AuthError, Capability, Fetchable, FetchableSource, Site, Stats};

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

/// This describes the Aeroscope "site" which is the PC we have here at the EIH
/// ///
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Aeroscope {
    /// Describe the different features of the source
    pub feature: Capability,
    /// Input formats
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
}

impl Aeroscope {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("aeroscope::new");

        // Set some reasonable defaults
        //
        Aeroscope {
            feature: Capability::Fetch,
            format: Format::Aeroscope,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            token: "".to_owned(),
        }
    }

    /// Load our site details from what is in the configuration file
    ///
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        self.format = Format::from_str(&site.format).unwrap();
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
        self.get = site.route("get").unwrap().to_owned();
        self
    }

    pub fn build(&mut self) -> Self {
        self.clone()
    }

    pub fn source(&self) -> FetchableSource {
        FetchableSource::Aeroscope(self.clone())
    }
}

impl Default for Aeroscope {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Aeroscope {
    fn name(&self) -> String {
        "aeroscope".to_string()
    }

    /// Authenticate to the site with login/password and return a token
    ///
    #[tracing::instrument(skip(self))]
    async fn authenticate(&self) -> Result<String, AuthError> {
        trace!("aeroscope::authenticate({:?})", &self.login);

        // Prepare our submission data
        //
        let cred = Credentials {
            username: self.login.clone(),
            password: self.password.clone(),
        };

        // fetch token
        //
        let url = format!("{}{}", self.base_url, self.token);
        trace!("Fetching token through {}…", url);

        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .json(&cred)
            .send()
            .await
            .map_err(|e| AuthError::HTTP(e.to_string()))?;

        let resp = resp
            .text()
            .await
            .map_err(|_| AuthError::Retrieval(cred.username.clone()))?;
        let res: Token =
            serde_json::from_str(&resp).map_err(|e| AuthError::Decoding(e.to_string()))?;

        debug!("res={:?}", res);
        Ok(res.access_token)
    }

    /// Fetch actual data from the site as a long String.
    ///
    #[tracing::instrument(skip(self))]
    async fn fetch(&self, out: Sender<String>, token: &str, _args: &str) -> Result<Stats> {

        // Use the token to authenticate ourselves
        //
        let url = format!("{}{}", self.base_url, self.get);
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header("content-type", "application/json")
            .header("user-agent", format!("{}/{}", crate_name!(), crate_version!()))
            .body(json!(&token).to_string())
            .send()
            .await
            .map_err(|e| AuthError::HTTP(e.to_string()))?;


        let resp = resp.text().await?;

        debug!("{} bytes read. ", resp.len());
        // Send statistics
        //
        let stats = Stats {
            tm: Utc::now().timestamp() as u64,
            pkts: 1u32,
            bytes: resp.len() as u64,
            ..Default::default()
        };

        let _ = out.send(resp)?;
        Ok(stats)
    }

    /// Returns the site's input formats
    ///
    fn format(&self) -> Format {
        Format::Aeroscope
    }
}
