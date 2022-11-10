//! ASD site specifics
//!
//! Phases:
//! 1. use login & password submitted to get a token
//! 2. use location & time data to restrict data set, submitted with token
//! 3. the answer is a filename and the data.  Currently `aeroscope-CDG.sh` fetch
//!    the data twice as it is requesting the specific filename returned but the
//!    data is already in the first call!
//!
//! Format is different from the json obtained from the actual Aeroscope system
//!
//! This implement the `Fetchable` trait described in `site/mod.rs`.
//!

use anyhow::Result;
use clap::{crate_name, crate_version};
use csv::ReaderBuilder;
use log::{debug, error, trace};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::format::{asd, Cat21, Format};
use crate::site::{Fetchable, Site};

/// Asd represent what is needed to connect & auth to and fetch data from the ASD main site.
///
#[derive(Clone, Debug)]
pub struct Asd {
    /// Input format
    pub format: Format,
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
    /// reqwest blocking client
    pub client: Client,
}

impl Asd {
    pub fn new() -> Self {
        Asd {
            format: Format::None,
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
                self.token = token.to_owned();
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

impl Default for Asd {
    fn default() -> Self {
        Self::new()
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
        let url = format!("{}{}", self.base_url, self.get);
        debug!("Fetching data through {}…", url);
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
        let res: Content = serde_json::from_str(&resp)?;
        debug!("{:?}", res);
        Ok(res.content)
    }

    fn process(&self, input: String) -> Result<Vec<Cat21>> {
        let mut rdr = ReaderBuilder::new()
            .flexible(true)
            .from_reader(input.as_bytes());

        let res: Vec<_> = rdr
            .records()
            .inspect(|f| println!("res={:?}", f.as_ref().unwrap()))
            .enumerate()
            .inspect(|(n, f)| println!("res={:?}-{:?}", n, f))
            .map(|(cnt, rec)| {
                let rec = rec.unwrap();
                debug!("rec={:?}", rec);
                let line: asd::Asd = rec.deserialize(None).unwrap();
                let mut line = Cat21::from(&line);
                line.rec_num = cnt;
                line
            })
            .collect();
        debug!("res={:?}", res);
        Ok(res)
    }

    fn format(&self) -> Format {
        Format::Asd
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

impl Default for Token {
    fn default() -> Self {
        Token {
            token: "".to_owned(),
            gjrt: "".to_owned(),
            expired_at: 0i64,
            roles: vec![],
            name: "John Doe".to_owned(),
            supervision: None,
            lang: "en".to_owned(),
            status: "".to_owned(),
            email: "john.doe@example.net".to_owned(),
            airspace_admin: None,
            homepage: "https://example.net".to_owned(),
        }
    }
}

/// Actual data when getting filteredlocations, it is json with the filename but also
/// the actual content so no need to fetch the named file.
///
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Content {
    /// Filename of the generated data file
    file_name: String,
    /// Actual CSV content
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    use httpmock::prelude::*;
    use serde_json::json;

    #[test]
    fn test_get_asd_token() {
        let server = MockServer::start();
        let token = Token {
            token: "FOOBAR".to_string(),
            ..Default::default()
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
        let site = Asd {
            format: Format::Asd,
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
