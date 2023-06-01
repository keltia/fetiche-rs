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

use std::io::{stderr, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{io, thread, time};

use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{crate_name, crate_version};
use log::{debug, trace};
use mini_moka::sync::Cache;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;

use fetiche_formats::{Format, StateList};

use crate::{http_get_basic, Fetchable, Filter, Streamable};
use crate::{Auth, Site};

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

/// Credentials to submit to the site to get the token
///
#[derive(Debug, Serialize)]
struct Credentials {
    /// Email as username
    username: String,
    /// Password
    password: String,
}

impl Opensky {
    pub fn new() -> Self {
        Opensky {
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
    pub fn load(&mut self, site: &Site) -> &mut Self {
        self.format = site.format.as_str().into();
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
    /// All credentials are passed every time we call the API so return a fake token
    ///
    fn authenticate(&self) -> anyhow::Result<String> {
        trace!("fake token retrieval");
        Ok(format!("{}:{}", self.login, self.password))
    }

    /// Single call API
    ///
    fn fetch(&self, out: &mut dyn Write, token: &str, args: &str) -> Result<()> {
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
            Filter::Stream { .. } | Filter::None => None,
        };

        let url = match tm {
            Some(tm) => format!("{}?{}", url, tm),
            _ => url,
        };
        trace!("FetchURL: {}", url);

        let resp = http_get_basic!(self, url, login, password)?;

        debug!("{:?}", &resp);

        // Check status
        //
        match resp.status() {
            StatusCode::OK => {
                trace!("OK");
            }
            code => {
                let h = &resp.headers();
                return Err(anyhow!("Error({}): {:?}", code, h));
            }
        }

        trace!("Fetching raw data");
        let resp = resp.text()?;
        write!(out, "{}", resp)?;
        Ok(())
    }

    fn format(&self) -> Format {
        Format::Opensky
    }
}

impl Streamable for Opensky {
    /// All credentials are passed every time we call the API so return a fake token
    ///
    fn authenticate(&self) -> Result<String> {
        trace!("fake token retrieval");
        Ok(format!("{}:{}", self.login, self.password))
    }

    /// The main stream function
    ///
    /// The cache might be overkill but:
    /// - it is easy to code and use
    /// - it can't hurt
    ///
    /// Right now it runs until killed by Ctrl+C, the timer expire (if set) or an API error.
    /// We may need to add a "retry until death or timeout" in order to keep trying (with a
    /// backoff timer I guess?) when encountering an API error.
    ///
    /// FIXME: should we have a thread for the timeout or the worker?  In the latter case, as soon
    ///        as the timer expire, we cancel the thread.  Now, we exit() from the thread.
    ///
    fn stream(&self, out: &mut dyn Write, token: &str, args: &str) -> Result<()> {
        trace!("opensky::stream");

        let mut stream_duration = 0;
        let mut stream_delay = 0;

        let cache = Cache::builder()
            .max_capacity(CACHE_SIZE)
            .time_to_idle(CACHE_IDLE)
            .time_to_live(CACHE_MAX)
            .build();

        let now = Utc::now().timestamp();

        let res: Vec<&str> = token.split(':').collect();
        let (login, password) = (res[0], res[1]);
        trace!("opensky::stream(as {}:{})", login, password);

        let url = format!("{}{}", self.base_url, self.get);
        trace!("Streaming data from {}…", url);

        // FIXME: we can have only one argument
        //
        let args = Filter::from(args);
        let tm = match args {
            Filter::Stream {
                duration,
                delay,
                from,
            } => {
                // default is 0
                stream_duration = duration;
                // default is 1000ms
                stream_delay = delay;

                if from == 0 {
                    None
                } else {
                    let start = if now - from > MAX_INTERVAL {
                        now - MAX_INTERVAL
                    } else {
                        from
                    };

                    // API takes 32-bit timestamp
                    //
                    let start: i32 = start.try_into().unwrap();
                    Some(format!("time={}", start))
                }
            }
            Filter::Keyword { name, value } => Some(format!("{}={}", name, value)),
            _ => None,
        };

        let url = match tm {
            Some(tm) => format!("{}?{}", url, tm),
            _ => url,
        };

        writeln!(
            stderr(),
            r##"
StreamURL: {}
Duration {}s with {}ms delay and cache with {} entries for {}s

<number>: data packet / ".": no traffic / "*": cache hit
        "##,
            url,
            stream_duration,
            stream_delay,
            CACHE_SIZE,
            CACHE_IDLE.as_secs(),
        )?;

        // Infinite loop until we get cancelled or timeout expire
        // self.duration is 0 -> infinite
        // self.duration is N -> run for N secs
        //
        let term = Arc::new(AtomicBool::new(false));

        // Setup signals
        //
        // NOTE: SIGINT must be issued twice to immediately stop, not sure is it needed.
        //
        for sig in TERM_SIGNALS {
            flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term))?;
            flag::register(*sig, Arc::clone(&term))?;
        }

        // Launch it!
        //
        while !term.load(Ordering::Relaxed) {
            trace!("Starting stream loop");

            // Now wait for Ctrl-C or timer expire
            //
            if stream_duration != 0 {
                // Timer set?
                //
                let d = stream_duration;
                thread::spawn(move || {
                    if d != 0 {
                        thread::sleep(time::Duration::from_secs(d as u64));

                        // FIXME: This is a library, libraries should never quit a process like
                        //        we do now.  We should return a `FINISHED` state.
                        //
                        std::process::exit(0);
                    }
                });
                trace!("end of sleep");
            }
            // Go!
            //
            let url = &url.clone();
            let login = &self.login.clone();
            let password = &self.password.clone();

            loop {
                let resp = http_get_basic!(self, url, login, password)?;

                debug!("{:?}", &resp);

                // Check status
                //
                match resp.status() {
                    StatusCode::OK => {
                        trace!("OK");
                    }
                    code => {
                        let h = &resp.headers();
                        return Err(anyhow!("Error({}): {:?}", code, h));
                    }
                }

                let resp = resp.text()?;

                // Retrieve answer and look into it, if answer was empty this should be rather fast
                //
                let sl: StateList = serde_json::from_str(&resp)?;

                // Check whether data was returned
                //
                if sl.states.is_some() {
                    // Check whether we've seen it before
                    //
                    match cache.get(&sl.time) {
                        // We have seen it, loop
                        //
                        Some(_time) => {
                            write!(stderr(), "*")?;
                            continue;
                        }

                        // No, write it and cache its `time`
                        //
                        _ => {
                            write!(stderr(), "{},", sl.time)?;
                            write!(out, "{}", resp)?;
                            out.flush()?;
                            cache.insert(sl.time, true);
                        }
                    }
                } else {
                    // Are there still entries?  If no, then we have only empty traffic for CACHE_MAX.
                    //
                    if cache.entry_count() == 0 {
                        write!(stderr(), "No traffic, waiting for 2s.");
                        thread::sleep(Duration::from_secs(2_u64));
                    } else {
                        write!(stderr(), ".")?;
                    }
                }

                // Whatever happened, sleep for to avoid CPU/network overload
                if stream_delay != 0 {
                    thread::sleep(Duration::from_millis(stream_delay as u64));
                }
            }
        }
        Ok(())
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
