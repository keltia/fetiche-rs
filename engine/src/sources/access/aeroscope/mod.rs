//! Aeroscope site-specifics
//!
//! Phases:
//! 1. use the configured login & password to obtain a token
//! 2. use the token to get the data
//!
//! Data fetched is json and not csv but our struct in `formats/aeroscope.rs`  is compatible with
//! both, even flattening the different lat/long structs in a sensible way.
//!
//! This implement the `Fetchable` trait described in `site/lib`.
//!

mod fetch;
mod actors;

use std::str::FromStr;
use std::sync::mpsc::Sender;
use std::vec;

use clap::{crate_name, crate_version};
use eyre::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use fetiche_formats::Format;

use crate::site::Site;
use crate::{http_get_auth, http_post, AsyncFetchable, Auth, AuthError, Capability, FetchableSource};

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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Aeroscope {
    /// Describe the different features of the source
    pub features: Vec<Capability>,
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
            features: vec![Capability::Fetch, Capability::Read],
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
    #[tracing::instrument]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("aeroscope::load({site:?})");

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

#[async_trait::async_trait]
impl AsyncFetchable for Aeroscope {
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

        let resp = http_post!(self, url, &cred).await.map_err(|e| AuthError::HTTP(e.to_string()))?;
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
    async fn fetch(&self, out: Sender<String>, token: &str, _args: &str) -> Result<()> {
        trace!("aeroscope::fetch");

        // Use the token to authenticate ourselves
        //
        let url = format!("{}{}", self.base_url, self.get);
        let resp = http_get_auth!(self, url, token).await?;
        let resp = resp.text().await?;

        debug!("{} bytes read. ", resp.len());
        Ok(out.send(resp)?)
    }

    /// Returns the site's input formats
    ///
    fn format(&self) -> Format {
        Format::Aeroscope
    }
}
