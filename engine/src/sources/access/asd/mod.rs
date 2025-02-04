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

use std::path::PathBuf;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};
use tracing::{error, trace, warn};

use fetiche_formats::Format;

use crate::{Auth, Capability, FetchableSource, Site};

mod fetch;
pub mod token;
mod actors;

pub use token::*;

/// Default token
const DEF_TOKEN: &str = "asd_default_token";

/// If no sources are defined, use these.
const DEF_SOURCES: &[&str] = &["as", "wi"];

/// Different types of source
///
#[derive(Clone, Debug, Deserialize, Serialize, EnumString, strum::Display, VariantNames)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Asd {
    /// Describe the different features of the source
    pub feature: Capability,
    /// Name of the site (site "foo" may use the same interface)
    pub site: String,
    /// Input formats
    pub format: Format,
    /// Base directory for tokens
    pub token_base: PathBuf,
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
    /// HTTP Client.
    #[serde(skip_serializing, skip_deserializing)]
    pub client: reqwest::Client,
}

impl Asd {
    #[tracing::instrument]
    pub fn new() -> Self {
        Asd {
            feature: Capability::Fetch,
            site: "NONE".to_string(),
            format: Format::Asd,
            token_base: PathBuf::new(),
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            token: "".to_owned(),
            get: "".to_owned(),
            client: reqwest::Client::new(),
        }
    }

    /// Load some data from the provided `Site` configuration into the `Asd` instance.
    ///
    /// This method initializes the `Asd` object with site-specific details, such as
    /// site name, format, base URL, credentials (if provided), and the route to fetch data.
    ///
    /// # Parameters
    /// - `site`: A `Site` reference containing the configuration details for the `Asd` instance.
    ///
    /// # Returns
    /// - A mutable reference to the current `Asd` instance, allowing method chaining.
    ///
    /// # Panics
    /// - Panics if the `auth` field in the `Site` does not contain a valid authentication configuration.
    ///
    #[tracing::instrument]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("asd::load");

        self.site = site.name.clone();
        self.format = Format::from_str(&site.format).unwrap();
        self.base_url = site.base_url.to_owned();
        self.token_base = site.token_base.clone();
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
                _ => {
                    error!("Invalid authentication parameters for {}.", site.name);
                    panic!("Invalid authentication parameters for {}.", site.name);
                }
            }
        }
        self.get = site.route("get").unwrap().to_owned();
        self
    }

    /// Finish the builder chain.
    ///
    pub fn build(&mut self) -> Self {
        self.clone()
    }

    /// Returns the `FetchableSource` representation of the current `Asd` instance.
    ///
    /// This method converts the `Asd` instance into a `FetchableSource`.
    /// A `FetchableSource` is used to represent data sources that can be
    /// fetched in a uniform and standardized way through `enum_dispatch`.
    ///
    /// # Returns
    ///
    /// - A `FetchableSource` that represents the current `Asd` instance.
    ///
    pub fn source(&self) -> FetchableSource {
        FetchableSource::from(self.clone())
    }
}

impl From<&Site> for Asd {
    fn from(s: &Site) -> Self {
        let mut asd = Asd::new().load(s).build();

        asd.site = s.name.clone();
        asd.format = Format::from_str(&s.format).unwrap();
        asd.base_url = s.base_url.to_owned();
        asd.token_base = s.token_base.clone();
        if let Some(auth) = &s.auth {
            match auth {
                Auth::Token {
                    token,
                    login,
                    password,
                } => {
                    asd.token = token.to_owned();
                    asd.login = login.to_owned();
                    asd.password = password.to_owned();
                }
                _ => panic!("nope"),
            }
        }
        asd.get = s.route("get").unwrap().to_owned();
        asd.clone()
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
#[cfg(feature = "json")]
fn into_ndjson(resp: &str) -> eyre::Result<String> {
    use serde_json::json;
    use tracing::debug;

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
