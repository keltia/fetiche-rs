//! OpenSky (.org) specific data
//!

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{io, thread, time};

use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{crate_name, crate_version};
use log::{debug, trace};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;

use fetiche_formats::{Cat21, Format, StateList};

use crate::{http_get_basic, Fetchable, Filter, Streamable};
use crate::{Auth, Site};

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
            format: Format::None,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            client: Client::new(),
            duration: 0,
        }
    }

    /// Load some data from the configuration file
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
        self.get = site.route("get").unwrap().to_owned();
        self
    }
}

impl Default for Opensky {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Opensky {
    fn authenticate(&self) -> anyhow::Result<String> {
        trace!("fake token retrieval");
        Ok(format!("{}:{}", self.login, self.password))
    }

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

    fn to_cat21(&self, input: String) -> Result<Vec<Cat21>> {
        let sl: StateList = serde_json::from_str(&input)?;
        let res = if let Some(res) = &sl.states {
            debug!("res={:?}", res);
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
            res
        } else {
            vec![]
        };
        debug!("res={:?}", res);
        Ok(res)
    }

    fn format(&self) -> Format {
        Format::Opensky
    }
}

impl Streamable for Opensky {
    fn authenticate(&self) -> anyhow::Result<String> {
        trace!("fake token retrieval");
        Ok(format!("{}:{}", self.login, self.password))
    }

    fn stream(&mut self, out: &mut dyn Write, token: &str, args: &str) -> Result<()> {
        let res: Vec<&str> = token.split(':').collect();
        let (login, password) = (res[0], res[1]);
        trace!("opensky::stream(as {}:{})", login, password);

        let url = format!("{}{}", self.base_url, self.get);
        trace!("Streaming data from {}…", url);

        // FIXME: we can have only one argument
        //
        let args: Filter = args.into();
        let tm = match args {
            Filter::Stream { from, duration } => {
                let now = Utc::now().timestamp() as i32;
                self.duration = duration;
                Some(format!("time={}", now))
            }
            Filter::Keyword { name, value } => Some(format!("{}={}", name, value)),
            _ => None,
        };

        let url = match tm {
            Some(tm) => format!("{}?{}", url, tm),
            _ => url,
        };
        trace!("StreamURL: {} for {} secs", url, self.duration);

        // Infinite loop until we get cancelled or timeout expire
        // self.duration is 0 -> infinite
        // self.duration is N -> run for N secs
        //
        let term = Arc::new(AtomicBool::new(false));

        // Setup signals
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
            if self.duration != 0 {
                // Timer set
                //
                let duration = self.duration;
                let t = thread::spawn(move || {
                    thread::sleep(time::Duration::from_secs(duration as u64))
                });
                t.join().unwrap();
                break;
            }
            // Go!
            //
            loop {
                write!(io::stderr(), ".")?;

                let url = &url.clone();
                let login = &self.login.clone();
                let password = &self.password.clone();
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
                write!(out, "{}", resp)?;
                out.flush()?;
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
#[derive(Debug, Serialize, Deserialize)]
struct Args {
    lamin: f32,
    lomin: f32,
    lamax: f32,
    lomax: f32,
}
