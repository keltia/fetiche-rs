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

mod token;

use std::fs;
use std::io::{BufReader, Write};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::Sender;

use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use clap::{crate_name, crate_version};
use datafusion::arrow::csv::WriterBuilder;
use datafusion::arrow::util::pretty::print_batches;
use datafusion::common::file_options::csv_writer::CsvWriterOptions;
use datafusion::common::parsers::CompressionTypeVariant;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::*;
use eyre::Report;
use eyre::{eyre, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use snafu::prelude::*;
use strum::{EnumString, VariantNames};
use tap::Tap;
use tempfile::{Builder, tempdir};
use tokio::runtime::Runtime;
use tracing::{debug, error, trace, warn};

use fetiche_formats::{Format, Asd as FAsd};

use crate::filter::Filter;
use crate::site::Site;
use crate::{http_post, Auth, Capability, Fetchable, Sources, Token};

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

    pub fn fetch_token(&self) -> Result<AsdToken> {
        trace!("fetching token from site");

        // Prepare our submission data
        //
        let cred = Credentials {
            email: self.login.clone(),
            password: self.password.clone(),
        };

        // fetch token from site
        //
        let url = format!("{}{}", self.base_url, self.token);
        trace!("Fetching token through {}…", url);
        let resp = http_post!(self, url, &cred)?;
        trace!("resp={:?}", resp);
        let resp = resp.text()?;
        let res: AsdToken = serde_json::from_str(&resp)?;
        trace!("token={}", res.token);

        Ok(res)
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

impl Fetchable for Asd {
    fn name(&self) -> String {
        self.site.to_string()
    }

    /// Authenticate to the site using the supplied credentials and get a token
    ///
    #[tracing::instrument(skip(self))]
    fn authenticate(&self) -> Result<String> {
        trace!("authenticate as ({:?})", &self.login);

        // Retrieve token from storage
        //
        // Use `<token>-<email>` to allow identity-based tokens
        //
        let res = match Token::retrieve(&self.login) {
            Ok(token) => {
                token.token
            }
            Err(e) => {
                let tok = self.fetch_token()?;
                let _ = tok.store()?;
                tok.token
            }
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

        let data = prepare_asd_data(data);
        debug!("data={}", &data);

        // use token
        //
        let url = format!("{}{}", self.base_url, self.get);
        trace!("Fetching data through {}…", url);

        // http_post_auth!() macro seems to be disturbing it.
        //
        let resp = self
            .client
            .clone()
            .post(url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", token))
            .body(data)
            .tap(|r| debug!("req={:?}", r))
            .send()?;

        debug!("raw resp={:?}", &resp);

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
                let errtxt = percent_decode(h["x-debug-exception"].as_bytes()).decode_utf8()?;
                let errfile =
                    percent_decode(h["x-debug-exception-file"].as_bytes()).decode_utf8()?;
                return Err(eyre!("Error({}): {} in {}", code, errtxt, errfile));
            }
        }

        // What we receive is an anonymous JSON object containing the filename and CSV content.
        //
        let resp = resp.text()?;
        trace!("resp={}", resp);
        let data: Payload = serde_json::from_str(&resp)?;

        trace!("Fetched {}", data.filename);

        // Save into a temp directory
        //
        let temp = tempdir()?;
        trace!("save {} into {}", data.filename, temp.path().to_str().unwrap());
        let fname = temp.path().join(PathBuf::from(data.filename));
        fs::write(&fname, &data.content)?;

        // Create tokio runtime
        //
        let rt = Runtime::new()?;

        let fname = fname.clone().to_string_lossy().clone().to_string();

        rt.block_on(async {
            update_time(&fname).await.unwrap();
        });
        let res = fs::read_to_string(&fname)?;

        // Now we must fixup the data by inserting the missing timestamp
        //
        Ok(out.send(res)?)
    }

    /// Return the site's input formats
    ///
    fn format(&self) -> Format {
        Format::Asd
    }
}

/// This is an async function that read the csv downloaded from ASD, add the "time" column as a
/// UNIX timestamp (u32) and save the resulting CSV.
///
/// Raw CSV will then be push to the next stage of the pipeline.
///
/// async because the datafusion API requires it.
///
async fn update_time(fname: &str) -> Result<()> {
    // Load out file in datafusion
    //
    let ctx = SessionContext::new();
    ctx.register_csv("drones", fname, CsvReadOptions::default()).await?;

    debug!("Reading {}, adding column time", fname);
    let df = ctx.sql("SELECT *,CAST(date_part('epoch', timestamp) AS int) AS time FROM drones").await?;

    let new = Builder::new().suffix(".csv").tempfile()?;
    debug!("Writing result into {}", new.path().to_str().unwrap());
    let a = df.write_csv(new.path().to_str().unwrap(), DataFrameWriteOptions::default(), None).await?;

    debug!("Rename into {}", fname);
    Ok(tokio::fs::rename(new.path(), fname).await?)
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

