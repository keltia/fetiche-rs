//! OpenSky (.org) specific code
//!
//! There are two trait implementations:
//! - `Fetchable`
//! - `Streamable`
//!
//! The `/states/own` endpoint can be polled several times and it always return a specific
//! `StateList` for which `time` is both timestamp and index.
//!
//! if two `StateList`s have the same `time`, there are the same.
//!
//! So now we cache them.
//!
//! FIXME: use a similar pattern as Senhive with actors.

mod actors;
mod stream;

use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{thread, time};

use chrono::Utc;
use clap::{crate_name, crate_version};
use eyre::{eyre, Result};
use mini_moka::sync::{Cache, ConcurrentCacheExt};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use tracing::{debug, error, info, trace};

use fetiche_formats::{Format, StateList};

use crate::{http_get_basic, Auth, Capability, Fetchable, Filter, Stats, Streamable};
use crate::{AuthError, Site};

/// We can go back only 1h in Opensky API
const MAX_INTERVAL: i64 = 3600;

/// Expiration after insert/get
const CACHE_IDLE: Duration = Duration::from_secs(20);
/// Expiration after insert
const CACHE_MAX: Duration = Duration::from_secs(60);
/// Cache max entries
const CACHE_SIZE: u64 = 20;

/// This si the Opensky client/source struct.
///
/// FIXME: this had only the "get" route (which will be "stream" for the streamable part.
///        this is confusing and incorrect.
///
#[derive(Clone, Debug)]
pub struct Opensky {
    /// Describe the different features of the source
    pub features: Vec<Capability>,
    /// Input formats
    pub format: Format,
    /// Username
    pub login: String,
    /// Password
    pub password: String,
    /// Base site url taken from config
    pub base_url: String,
    /// Add this to `base_url` to fetch data
    pub get: String,
    /// reqwest blocking client
    pub client: Client,
    /// Running time (for streams)
    pub duration: i32,
}

#[allow(dead_code)]
/// This is the struct holding potential parameters to the API
///
#[derive(Debug, Serialize)]
struct Param {
    /// timestamp of the state vectors to be retrieved
    pub time: Option<u32>,
    /// One or more ICAO24 transponder address
    pub icao24: Option<Vec<String>>,
    /// One or more receiver IDs
    pub serials: Option<Vec<u32>>,
}

#[allow(dead_code)]
/// Credentials to submit to the site to get the token
///
#[derive(Debug, Serialize)]
struct Credentials {
    /// Email as username
    username: String,
    /// Password
    password: String,
}

/// Messages to send to the stats threads
///
#[derive(Clone, Debug, Serialize)]
enum StatMsg {
    Pkts,
    Bytes(u64),
    Hits,
    Miss,
    Empty,
    Error,
    Print,
    Exit,
}

impl Opensky {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("opensky::new");

        Opensky {
            features: vec![Capability::Fetch, Capability::Stream],
            format: Format::Opensky,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            client: Client::new(),
            duration: 0,
        }
    }

    /// Load some data from in-memory loaded config
    ///
    #[tracing::instrument]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("opensky::load");

        self.format = Format::from_str(&site.format).unwrap();
        self.base_url = site.base_url.to_owned();
        if let Some(auth) = &site.auth {
            match auth {
                Auth::Login {
                    username: login,
                    password,
                } => {
                    self.login = login.to_owned();
                    self.password = password.to_owned();
                }
                _ => panic!("nope"),
            }
        }
        // FIXME: should get the entire set of routes
        //
        self.get = site.route("stream").unwrap().to_owned();
        self
    }
}

impl Default for Opensky {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Opensky {
    fn name(&self) -> String {
        "opensky".to_string()
    }

    /// All credentials are passed every time we call the API so return a fake token
    ///
    #[tracing::instrument]
    fn authenticate(&self) -> Result<String, AuthError> {
        trace!("fake token retrieval");
        Ok(format!("{}:{}", self.login, self.password))
    }

    /// Single call API
    ///
    #[tracing::instrument]
    fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> Result<()> {
        trace!("opensky::fetch");

        let res: Vec<&str> = token.split(':').collect();
        let (login, password) = (res[0], res[1]);
        trace!("opensky::fetch(as {}:{})", login, password);

        let url = format!("{}{}", self.base_url, self.get);
        trace!("Fetching data from {}…", url);

        // FIXME: we can have only one argument
        //
        let args: Filter = args.into();
        let tm = match args {
            Filter::Interval { begin, .. } => {
                let now = begin.timestamp() as i32;
                Some(format!("time={}", now))
            }
            Filter::Duration(d) => {
                let now = Utc::now().timestamp() as i32;
                Some(format!("time={}", now - d))
            }
            Filter::Keyword { name, value } => Some(format!("{}={}", name, value)),
            _ => None,
        };

        let url = match tm {
            Some(tm) => format!("{}?{}", url, tm),
            _ => url,
        };
        trace!("FetchURL: {}", url);

        let client = self.client.clone();
        let resp = http_get_basic!(client, url, login, password)?;

        debug!("{:?}", &resp);

        // Check status
        //
        match resp.status() {
            StatusCode::OK => {
                trace!("OK");
            }
            code => {
                let h = &resp.headers();
                return Err(eyre!("Error({}): {:?}", code, h));
            }
        }

        trace!("Fetching raw data");
        let resp = resp.text()?;
        Ok(out.send(resp)?)
    }

    fn format(&self) -> Format {
        Format::Opensky
    }
}

/// Represent the area we want to get all from
///
/// FIXME: this is not handled
///
#[derive(Debug, Serialize, Deserialize)]
struct Args {
    lamin: f32,
    lomin: f32,
    lamax: f32,
    lomax: f32,
}
