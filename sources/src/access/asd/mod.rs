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

use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use eyre::Result;
use polars::datatypes::Int64Chunked;
use polars::prelude::{Column, IntoColumn};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};
use tracing::{error, trace, warn};

#[cfg(feature = "json")]
use tracing::debug;

use fetiche_formats::Format;

use crate::site::Site;
use crate::{Auth, AuthError, Capability};

#[cfg(feature = "json")]
use serde_json::json;

mod fetch;
pub mod token;

use crate::init::{init_sources_runtime, Context};
pub use token::*;

/// Default token
const DEF_TOKEN: &str = "asd_default_token";

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
#[derive(Clone, Debug)]
pub struct Asd {
    /// Describe the different features of the source
    pub features: Vec<Capability>,
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
    /// reqwest blocking client
    pub client: Client,
    /// supervisor and stats actors references
    pub ctx: Context,
}

impl Asd {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("asd::new");
        Asd::default()
    }

    /// Load some data from the configuration file
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
                _ => panic!("nope"),
            }
        }
        self.get = site.route("get").unwrap().to_owned();
        self
    }
    /// Return the content of named token
    ///
    #[tracing::instrument]
    pub fn retrieve(fname: &PathBuf) -> Result<String> {
        trace!("get_token from {fname:?}");
        if fname.exists() {
            Ok(fs::read_to_string(fname)?)
        } else {
            Err(AuthError::Retrieval(fname.to_string_lossy().to_string()).into())
        }
    }

    /// Store (overwrite) named token
    ///
    #[tracing::instrument]
    pub fn store(fname: &PathBuf, data: &str) -> Result<()> {
        let dir = fname.parent().unwrap();

        // Check token cache
        //
        if !dir.exists() {
            // Create it
            //
            trace!("create token store: {dir:?}");

            fs::create_dir_all(dir)?
        }
        trace!("store_token: {fname:?}");
        Ok(fs::write(fname, data)?)
    }

    /// Purge expired token
    ///
    #[tracing::instrument]
    pub fn purge(fname: &PathBuf) -> Result<()> {
        trace!("purge expired token in {fname:?}");

        Ok(fs::remove_file(fname)?)
    }
}

impl Default for Asd {
    fn default() -> Self {
        let ctx = match init_sources_runtime() {
            Ok(ctx) => ctx,
            Err(e) => {
                error!("Can not initialize sources: {e}");
                std::process::exit(1);
            }
        };
        Asd {
            features: vec![Capability::Fetch],
            site: "NONE".to_string(),
            format: Format::Asd,
            token_base: PathBuf::new(),
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            token: "".to_owned(),
            get: "".to_owned(),
            client: Client::new(),
            ctx,
        }
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

/// ASD is very sensitive to the date format, needs milli-secs.
///
fn prepare_asd_data(data: Param) -> String {
    let d_start = data.start_time.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let d_end = data.end_time.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    format!(
        "{{\"startTime\":\"{}\",\"endTime\":\"{}\",\"sources\":[\"as\",\"wi\"]}}",
        d_start, d_end
    )
}

/// Generate a UNIX timestamp from the non-standard date string used by Asd.
///
fn into_timestamp(col: &Column) -> Column {
    col.str()
        .unwrap()
        .into_iter()
        .map(|d: Option<&str>| d.map(|d: &str| dateparser::parse(d).unwrap().timestamp()))
        .collect::<Int64Chunked>()
        .into_column()
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
