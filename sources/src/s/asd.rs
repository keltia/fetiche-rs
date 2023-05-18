//! ASD site specifics
//!
//! Phases:
//! 1. use login & password submitted to get a token
//! 2. use location & time data to restrict data set, submitted with token
//! 3. the answer is a filename and the data.  Currently `aeroscope-CDG.sh` fetch
//!    the data twice as it is requesting the specific filename returned but the
//!    data is already in the first call!
//!
//! Format is different from the json obtained from the actual Aeroscope system but the `Asd` is
//! compatible with both CSV and JSON output from the site.
//!
//! This implement the `Fetchable` trait described in `site/lib`.
//!

use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use clap::{crate_name, crate_version};
use log::{debug, trace};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

use format_specs::Asd as InputFormat;
use format_specs::{Cat21, Format};

use crate::filter::Filter;
use crate::site::{Auth, Site};
use crate::{http_post, http_post_auth, Fetchable};

/// Different types of source
///
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, into = "String")]
enum Source {
    /// ADS-B
    Ab,
    /// OGN
    Og,
    /// Wifi (signalement InfoDrone)
    Wi,
    /// Aeroscope
    As,
    /// ASD (tracers)
    Ad,
    /// ASD (mobile app)
    Mo,
}

impl From<Source> for String {
    /// For serialization into json
    ///
    fn from(s: Source) -> Self {
        match s {
            Source::Ab => "ab",
            Source::Og => "og",
            Source::Wi => "wi",
            Source::As => "as",
            Source::Ad => "ad",
            Source::Mo => "mo",
        }
        .to_string()
    }
}

/// Credentials to submit to the site to get the token
///
#[derive(Debug, Serialize)]
struct Credentials {
    /// Email as username
    email: String,
    /// Password
    password: String,
}

/// Data to submit to get replay of journeys
///
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Param {
    /// Limit ourselves to this time interval beginning at
    start_time: NaiveDateTime,
    /// Limit ourselves to this time interval ending at
    end_time: NaiveDateTime,
    /// Source of data from ASD, see below `Source` enum.
    sources: Vec<Source>,
}

/// Asd represent what is needed to connect & auth to and fetch data from the ASD main site.
///
#[derive(Clone, Debug)]
pub struct Asd {
    /// Name of the site (site "foo" may use the same interface)
    pub site: String,
    /// Input formats
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
            site: "NONE".to_string(),
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
        self.site = site.name.clone().unwrap();
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
        self.get = site.route("get").unwrap().to_owned();
        self
    }
}

impl Default for Asd {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Asd {
    /// Authenticate to the site using the supplied credentials and get a token
    ///
    /// TODO: check whether $config/tokens/<source>.token exists and if yes, check
    ///       expiration date and possibly re-use it.
    ///
    fn authenticate(&self) -> Result<String> {
        //let curr_tok =
        // Prepare our submission data
        //
        trace!("Submit auth as {:?}", &self.login);
        let cred = Credentials {
            email: self.login.clone(),
            password: self.password.clone(),
        };

        // fetch token
        //
        let url = format!("{}{}", self.base_url, self.token);
        trace!("Fetching token through {}…", url);
        let resp = http_post!(self, url, &cred)?;
        trace!("resp={:?}", resp);
        let resp = resp.text()?;
        let res: Token = serde_json::from_str(&resp)?;
        trace!("token={}", res.token);
        Ok(res.token)
    }

    /// Fetch actual data using the aforementioned token
    ///
    fn fetch(&self, token: &str, args: &str) -> Result<String> {
        trace!("Submit parameters");

        let f: Filter = serde_json::from_str(args)?;

        // If we have a filter defined, extract times
        //
        let data = match f {
            Filter::Interval { begin, end } => Param {
                start_time: begin,
                end_time: end,
                sources: vec![Source::As, Source::Wi],
            },
            Filter::None => Param {
                start_time: NaiveDateTime::from_timestamp_opt(0i64, 0u32).unwrap(),
                end_time: NaiveDateTime::from_timestamp_opt(0i64, 0u32).unwrap(),
                sources: vec![Source::As, Source::Wi],
            },
        };

        debug!("param={:?}", data);
        debug!("json={}", json!(&data));

        // use token
        //
        let url = format!("{}{}", self.base_url, self.get);
        debug!("Fetching data through {}…", url);

        let resp = http_post_auth!(self, url, token, &data)?;

        debug!("{:?}", &resp);

        // Check status
        //
        match resp.status() {
            StatusCode::OK => {}
            code => {
                // This is highly ASD specific
                //
                use percent_encoding::percent_decode;
                trace!("error resp={:?}", resp);
                let h = &resp.headers();
                let errtxt = percent_decode(h["x-debug-exception"].as_bytes())
                    .decode_utf8()
                    .unwrap();
                return Err(anyhow!("Error({}): {}", code, errtxt));
            }
        }

        let resp = resp.text()?;
        Ok(resp)
    }

    /// Process every fetched data line and generate the `Cat21` result
    ///
    fn process(&self, input: String) -> Result<Vec<Cat21>> {
        debug!("Reading & transforming…");
        debug!("IN={:?}", input);
        let res: Vec<InputFormat> = serde_json::from_str(&input)?;

        debug!("rec={:?}", res);
        let res: Vec<_> = res
            .iter()
            .enumerate()
            .inspect(|(n, f)| debug!("f={:?}-{:?}", n, f))
            .map(|(cnt, rec)| {
                debug!("cnt={}/rec={:?}", cnt, rec);
                let mut line = Cat21::from(rec);
                line.rec_num = cnt;
                line
            })
            .collect();
        debug!("res={:?}", res);
        Ok(res)
    }

    /// Return the site's input formats
    ///
    fn format(&self) -> Format {
        Format::Asd
    }
}

/// Access token derived from username/password
///
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Token {
    /// The actual token
    token: String,
    /// Don't ask
    gjrt: String,
    expired_at: i64,
    roles: Vec<String>,
    name: String,
    supervision: Option<String>,
    lang: String,
    status: String,
    email: String,
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

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;
    use serde_json::json;

    use crate::filter::Filter;

    use super::*;

    fn setup_asd(server: &MockServer) -> Asd {
        let client = Client::new();
        Asd {
            site: "NONE".to_string(),
            format: Format::Asd,
            login: "user".to_string(),
            password: "pass".to_string(),
            token: "/api/security/login".to_string(),
            base_url: server.base_url().clone(),
            get: "/api/journeys/filteredlocations/json".to_string(),
            client: client.clone(),
        }
    }

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
                .path("/api/security/login");
            then.status(200).body(&jtok);
        });

        let site = setup_asd(&server);
        let t = site.authenticate();

        m.assert();
        assert!(t.is_ok());
        assert_eq!("FOOBAR", t.as_ref().unwrap());
    }

    // #[test]
    // fn test_get_asd_fetch() {
    //     let server = MockServer::start();
    //     let filter = Filter::default();
    //     let filter = "{}".to_string();
    //     let token = "FOOBAR".to_string();
    //     let m = server.mock(|when, then| {
    //         when.method(POST)
    //             .header(
    //                 "user-agent",
    //                 format!("{}/{}", crate_name!(), crate_version!()),
    //             )
    //             .header("content-type", "application/json")
    //             .header("authorization", format!("Bearer {}", token))
    //             .path("/api/journeys/filteredlocations/json")
    //             .body(&filter);
    //         then.status(200).body("");
    //     });
    //
    //     let site = setup_asd(&server);
    //     dbg!(&site);
    //
    //     let t = "FOOBAR";
    //     let d = site.fetch(&t, &Filter::default().to_string());
    //
    //     m.assert();
    //     assert!(d.is_ok());
    // }
}
