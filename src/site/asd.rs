//! ASD site specifics
//!
//! Phases:
//! 1. use login & password submitted to get a token
//! 2. use location & time data to restrict data set, submitted with token
//! 3. the answer is a filename and the data.  Currently `aeroscope-CDG.sh` fetch
//!    the data twice as it is requesting the specific filename returned but the
//!    data is already in the first call!
//!
//! Format is different from the csv obtained from the actual Aeroscope system
//!

use anyhow::{anyhow, format_err, Result};
use clap::{crate_name, crate_version};
use log::{debug, error, trace};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::format::Source;
use crate::site::aeroscope::Aeroscope;
use crate::site::Fetchable;
use crate::{Config, Site};

#[derive(Clone, Debug)]
pub struct Asd {
    /// Input format
    pub format: Source,
    /// Username
    pub login: String,
    /// Password
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

const NAME: &str = "asd";

impl Asd {
    pub fn new() -> Self {
        Asd {
            format: Source::None,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            token: "".to_owned(),
            get: "".to_owned(),
            client: Client::new(),
        }
    }

    /// Load some data from the configuration file
    ///
    pub fn load(&mut self, cfg: &Config) -> &mut Self {
        match &cfg.sites[NAME] {
            Site::Login {
                format,
                base_url,
                login,
                password,
                token,
                get,
                ..
            } => {
                self.format = Source::from_str(format);
                self.base_url = base_url.to_owned();
                self.token = token.to_owned();
                self.get = get.to_owned();
                self.login = login.to_owned();
                self.password = password.to_owned();
            }
            _ => {
                error!("Missing config data for {NAME}")
            }
        }
        self
    }
}

impl Fetchable for Asd {
    fn authenticate(&self) -> Result<String> {
        // Prepare our submission data
        //
        trace!("Submit auth as {:?}", &self.login);
        let body = format!(
            "{{\"email\": \"{}\", \"password\": \"{}\"}}",
            self.login, self.password
        );

        // fetch token
        //
        let url = format!("{}{}", self.base_url, self.token);
        trace!("Fetching token through {}…", url);
        let resp = self
            .client
            .clone()
            .post(url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .body(body)
            .send();

        let resp = resp?.text()?;
        let res: Token = serde_json::from_str(&resp)?;
        debug!("{:?}", res);
        Ok(res.token)
    }

    /// Fetch actual data
    ///
    fn fetch(&self, token: &str) -> Result<String> {
        trace!("Submit parameters");
        let data = format!(
            "{{\"startTime\": \"'{}'\",\"endTime\": \"'{}}}'\",\"sources\": [\"as\",\"wi\"]}}",
            "", ""
        );

        // use token
        //
        let url = format!("{}{}", self.base_url, self.token);
        trace!("Fetching token through {}…", url);
        let resp = self
            .client
            .clone()
            .post(url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("authentication", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(data)
            .send();

        let resp = resp?.text()?;
        let res: Token = serde_json::from_str(&resp)?;
        debug!("{:?}", res);
        Ok(res.token)
    }

    fn format(&self) -> Source {
        Source::Asd
    }
}

/// Access token derived from username/password
///
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Token {
    /// The actual token
    token: String,
    /// Don't ask
    gjrt: String,
    #[serde(rename = "expiredAt")]
    expired_at: i64,
    roles: Vec<String>,
    name: String,
    supervision: Option<String>,
    lang: String,
    status: String,
    email: String,
    #[serde(rename = "airspaceAdmin")]
    airspace_admin: Option<String>,
    homepage: String,
}
