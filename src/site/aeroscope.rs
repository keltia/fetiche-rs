//! Aeroscope site-specifics
//!
//! Phases:
//! 1. use the configured login & password to obtain a token
//! 2. use the token to get the data
//!
//! Format is a CSV as Aeroscope
//!

use anyhow::Result;
use clap::{crate_name, crate_version};
use log::{debug, error, trace};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::format::Source;
use crate::site::Fetchable;
use crate::{Config, Site};

#[derive(Clone, Debug)]
pub struct Aeroscope {
    /// Input format
    pub format: Source,
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

const NAME: &str = "aeroscope";

impl Aeroscope {
    pub fn new() -> Self {
        Aeroscope {
            format: Source::None,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            token: "".to_owned(),
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
                self.get = get.to_owned();
                self.token = token.to_owned();
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

impl Fetchable for Aeroscope {
    /// Authenticate to the site with login/password and return a token
    ///
    fn authenticate(&self) -> Result<String> {
        // Prepare our submission data
        //
        trace!("Submit auth as {:?}", &self.login);
        let body = format!(
            "{{\"username\": \"{}\", \"password\": \"{}\"}}",
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
        Ok(res.access_token)
    }

    /// Fetch actual data from the site as a long String.
    ///
    fn fetch(&self, token: &str) -> Result<String> {
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
            .header("Authorization", format!("Bearer {}", token))
            .send()?
            .text()?;
        Ok(resp)
    }

    fn format(&self) -> Source {
        Source::Aeroscope
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

    use clap::{crate_name, crate_version};
    use httpmock::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::site::Site;

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

        let client = reqwest::blocking::Client::new();
        let site = Site::Login {
            format: "aeroscope".to_string(),
            login: "user".to_string(),
            auth: "aeroscope".to_string(),
            password: "pass".to_string(),
            token: "/login".to_string(),
            base_url: server.base_url().clone(),
            get: "/get".to_string(),
        };
        let t = site.auth(&client);

        m.assert();
        assert!(t.is_ok());
        assert_eq!("FOOBAR", t.as_ref().unwrap());
    }
}
