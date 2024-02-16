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
//! Switched from JSON to CSV to work around the size limit from the API ~50 MB
//!
//! [NDJSON]: https://en.wikipedia.org/wiki/NDJSON

use std::ops::Add;
use std::str::FromStr;
use std::sync::mpsc::Sender;

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use clap::{crate_name, crate_version};
use eyre::Report;
use eyre::{eyre, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use snafu::prelude::*;
use strum::EnumString;
use tracing::{debug, error, trace, warn};

use fetiche_formats::Format;

use crate::filter::Filter;
use crate::site::Site;
use crate::{http_post, http_post_auth, Auth, Capability, Fetchable, Sources};

/// Default token
const DEF_TOKEN: &str = "asd_default_token";

/// Different types of source
///
#[derive(Clone, Debug, Deserialize, Serialize, EnumString, strum::Display, strum::VariantNames)]
#[strum(serialize_all = "lowercase")]
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
    start_time: DateTime<Utc>,
    /// Limit ourselves to this time interval ending at
    end_time: DateTime<Utc>,
    /// Source of data from ASD, see below `Source` enum.
    sources: Vec<Source>,
}

/// Asd represent what is needed to connect & auth to and fetch data from the ASD main site.
///
#[derive(Clone, Debug)]
pub struct Asd {
    /// Describe the different features of the source
    pub features: Vec<Capability>,
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
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("asd::new");

        Asd {
            features: vec![Capability::Fetch],
            site: "NONE".to_string(),
            format: Format::Asd,
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
    #[tracing::instrument]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("asd::load");

        self.site = site.name.clone().unwrap();
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
}

impl Default for Asd {
    fn default() -> Self {
        Self::new()
    }
}

/// CSV payload from `.../filteredlocation`
///
#[derive(Debug, Deserialize)]
struct Payload {
    /// Filename if one need to fetch as a file.
    #[serde(rename = "fileName")]
    filename: String,
    /// CSV content is here already.
    content: String,
}

impl Fetchable for Asd {
    fn name(&self) -> String {
        self.site.to_string()
    }

    /// Authenticate to the site using the supplied credentials and get a token
    ///
    #[tracing::instrument(skip(self))]
    fn authenticate(&self) -> Result<String> {
        trace!("authenticate as ({:?})", &self.login);

        // Prepare our submission data
        //
        let cred = Credentials {
            email: self.login.clone(),
            password: self.password.clone(),
        };

        // Retrieve token from storage
        //
        // Use `<token>-<email>` to allow identity-based tokens
        //
        let fname = format!("{}-{}", DEF_TOKEN, self.login);
        let res = if let Ok(token) = Sources::get_token(&fname) {
            // Load potential token data
            //
            trace!("load stored token");
            let token: Token = match serde_json::from_str(&token) {
                Ok(token) => token,
                Err(_) => return Err(eyre!("Invalid/no token in {:?}", token)),
            };

            // Check stored token expiration date
            //
            let now: DateTime<Utc> = Utc::now();
            let tok_time: DateTime<Utc> = Utc.timestamp_opt(token.expired_at, 0).unwrap();
            if now > tok_time {
                // Should we delete it?
                //
                warn!("Stored token in {:?} has expired, deleting!", fname);
                match Sources::purge_token(&fname) {
                    Ok(()) => (),
                    Err(e) => error!("Can not remove token: {}", e.to_string()),
                };
                return Err(Report::from(TokenError::Expired));
            }
            trace!("token is valid");
            token.token
        } else {
            trace!("no token");

            // fetch token from site
            //
            let url = format!("{}{}", self.base_url, self.token);
            trace!("Fetching token through {}…", url);
            let resp = http_post!(self, url, &cred)?;
            trace!("resp={:?}", resp);
            let resp = resp.text()?;
            let res: Token = serde_json::from_str(&resp)?;
            trace!("token={}", res.token);

            // Write fetched token in `tokens` (unless it is during tests)
            //
            #[cfg(not(test))]
            Sources::store_token(&fname, &resp)?;

            res.token
        };

        // Return final token
        //
        Ok(res)
    }

    /// Fetch actual data using the aforementioned token
    ///
    #[tracing::instrument(skip(self))]
    fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> Result<()> {
        trace!("asd::fetch");

        const DEF_SOURCES: &[Source] = &[Source::As, Source::Wi];

        let f: Filter = serde_json::from_str(args)?;

        // If we have a filter defined, extract times
        //
        let data = match f {
            Filter::Duration(d) => Param {
                start_time: NaiveDateTime::default().and_utc(),
                end_time: NaiveDateTime::default()
                    .and_utc()
                    .add(Duration::seconds(d as i64)),
                sources: DEF_SOURCES.to_vec(),
            },
            Filter::Interval { begin, end } => Param {
                start_time: begin,
                end_time: end,
                sources: DEF_SOURCES.to_vec(),
            },
            _ => Param {
                start_time: DateTime::<Utc>::MIN_UTC,
                end_time: DateTime::<Utc>::MIN_UTC,
                sources: DEF_SOURCES.to_vec(),
            },
        };

        debug!("json={}", json!(&data));

        // use token
        //
        let url = format!("{}{}", self.base_url, self.get);
        trace!("Fetching data through {}…", url);

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
                let h = resp.headers();
                let errtxt = percent_decode(h["x-debug-exception"].as_bytes())
                    .decode_utf8()
                    .unwrap();
                return Err(eyre!("Error({}): {}", code, errtxt));
            }
        }

        // What we receive is an anonymous JSON object containing the filename and CSV content.
        //
        let resp = resp.text()?;
        let data: Payload = serde_json::from_str(&resp)?;

        trace!("Fetched {}", data.filename);
        Ok(out.send(data.content)?)
    }

    /// Return the site's input formats
    ///
    fn format(&self) -> Format {
        Format::Asd
    }
}

/// ASD is sending us an anonymous JSON array
///
/// This is less easy to use later on so we convert it into [NDJSON]
///
/// This is analogous to running the following (without the `time` fix):
/// ```text
/// jq --compact-output '.[]' < today.json > lines.json
/// ```
///
fn into_ndjson(resp: &str) -> Result<String> {
    let data: Vec<fetiche_formats::Asd> = serde_json::from_str(resp)?;
    let res = data
        .iter()
        .map(|r| {
            debug!("r={:?}", r);
            // Fix timestamp while we are here
            //
            let r = r.fix_tm().unwrap();
            json!(&r).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(res)
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
    /// Expiration date
    expired_at: i64,
    roles: Vec<String>,
    /// Fullname
    name: String,
    supervision: Option<String>,
    lang: String,
    status: String,
    email: String,
    airspace_admin: Option<String>,
    homepage: String,
}

/// Custom error type for tokens, allow us to differentiate between errors.
///
#[derive(Debug, PartialEq, Snafu)]
pub enum TokenError {
    #[snafu(display("Token expired"))]
    Expired,
    #[snafu(display("Invalid token"))]
    Invalid,
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
    use env_logger;
    use httpmock::prelude::*;
    use serde_json::json;

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn setup_asd(server: &MockServer) -> Asd {
        init();
        let client = Client::new();
        Asd {
            features: vec![Capability::Fetch],
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
        let now = Utc::now().timestamp() + 3600i64;
        let token = Token {
            token: "FOOBAR".to_string(),
            expired_at: now,
            ..Default::default()
        };

        let jtok = json!(token).to_string();
        let cred = Credentials {
            email: "user".to_string(),
            password: "pass".to_string(),
        };
        let cred = json!(cred).to_string();
        let m = server.mock(|when, then| {
            when.method(POST)
                .header(
                    "user-agent",
                    format!("{}/{}", crate_name!(), crate_version!()),
                )
                .header("content-type", "application/json")
                .body(&cred)
                .path("/api/security/login");
            then.status(200).body(&jtok);
        });

        let site = setup_asd(&server);
        let t = site.authenticate();
        dbg!(&t);
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
