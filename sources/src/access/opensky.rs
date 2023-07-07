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

use std::fmt::{Display, Formatter};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{thread, time};

use anyhow::{anyhow, Result};
use chrono::Utc;
use clap::{crate_name, crate_version};
use mini_moka::sync::{Cache, ConcurrentCacheExt};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use tracing::{debug, error, info, trace};

use fetiche_formats::{Format, StateList};

use crate::Site;
use crate::{http_get_basic, Auth, Capability, Fetchable, Filter, Streamable};

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

/// Statistics gathering struct
///
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Stats {
    pub tm: u64,
    pub pkts: u32,
    pub bytes: u64,
    pub hits: u32,
    pub miss: u32,
    pub empty: u32,
    pub err: u32,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            tm: 0_u64,
            pkts: 0,
            bytes: 0_u64,
            hits: 0,
            miss: 0,
            empty: 0,
            err: 0,
        }
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "time={}s pkts={} bytes={} hits={} miss={} empty={} errors={}",
            self.tm, self.pkts, self.bytes, self.hits, self.miss, self.empty, self.err
        )
    }
}

/// Messages to send to the stats threads
///
#[derive(Clone, Debug, Serialize)]
pub(crate) enum StatMsg {
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
    fn name(&self) -> String {
        "opensky".to_string()
    }

    /// All credentials are passed every time we call the API so return a fake token
    ///
    #[tracing::instrument]
    fn authenticate(&self) -> Result<String> {
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
            Filter::Stream { .. } | Filter::None => None,
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
                return Err(anyhow!("Error({}): {:?}", code, h));
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

impl Streamable for Opensky {
    fn name(&self) -> String {
        "opensky".to_string()
    }

    /// All credentials are passed every time we call the API so return a fake token
    ///
    fn authenticate(&self) -> Result<String> {
        trace!("fake token retrieval");
        Ok(format!("{}:{}", self.login, self.password))
    }

    /// The main stream function
    ///
    /// Right now it runs until killed by Ctrl+C or the timer expire (if set).
    ///
    /// API Error are currently ignored after waiting for some time.  The server is not
    /// stable enough to consider fatal errors (5xx) as real.  It will recover even after
    /// a 502.
    ///
    /// The cache might be overkill because keeping only the last timestamp might be enough but:
    /// - it is easy to code and use
    /// - it helps determining whether we had lack of traffic for a longer time if we have no
    ///   cached entries
    ///
    #[tracing::instrument]
    fn stream(&self, out: Sender<String>, token: &str, args: &str) -> Result<()> {
        trace!("opensky::stream");

        let mut stream_duration = 0;
        let mut stream_delay = 1000;

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
                stream_duration = duration;
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

        info!(
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
        );

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

        // out as a `dyn Write` is not `Send` so we can not use it within a thread.  Use channels
        // to work around this.
        //
        let (tx, rx) = channel::<String>();

        // Timer set?  If yes, launch a sleeper thread
        //
        if stream_duration != 0 {
            trace!("setup wakeup alarm");

            let d = stream_duration;
            let tx1 = tx.clone();
            thread::spawn(move || {
                trace!("alarm set to {}s", d);
                thread::sleep(time::Duration::from_secs(d as u64));
                tx1.send("TIMEOUT".to_string()).unwrap();
            });
            trace!("end of sleep");
        }

        // reqwest::blocking::Client
        //
        let client = self.client.clone();

        let login = self.login.clone();
        let password = self.password.clone();

        // Launch stat gathering thread.
        //
        let (st_tx, st_rx) = channel::<StatMsg>();
        let stat_id = thread::spawn(move || {
            trace!("stats::thread");

            let start = Instant::now();
            let mut stats = Stats::default();
            while let Ok(msg) = st_rx.recv() {
                match msg {
                    StatMsg::Pkts => stats.pkts += 1,
                    StatMsg::Hits => stats.hits += 1,
                    StatMsg::Miss => stats.miss += 1,
                    StatMsg::Empty => stats.empty += 1,
                    StatMsg::Error => stats.err += 1,
                    StatMsg::Bytes(n) => stats.bytes += n,
                    StatMsg::Print => {
                        stats.tm = start.elapsed().as_secs();
                        eprintln!("Stats: {}", stats)
                    }
                    // The end
                    StatMsg::Exit => {
                        stats.tm = start.elapsed().as_secs();
                        eprintln!("\nSession: {}", stats);
                        break;
                    }
                }
            }
            trace!("end of stats thread");
            eprintln!("Stats: {}", stats);
        });

        // Launch a thread that sleep for 30s then ask for statistics
        //
        let disp_tx = st_tx.clone();
        let disp_id = thread::spawn(move || {
            trace!("stats::display");
            loop {
                thread::sleep(Duration::from_secs(30_u64));
                disp_tx.send(StatMsg::Print).expect("ping");
            }
        });

        // Worker thread1
        //
        let stat_tx = st_tx.clone();
        let work_id = thread::spawn(move || {
            trace!("Starting worker thread");

            // Cache is local to the worker thread
            //
            let cache = Cache::builder()
                .max_capacity(CACHE_SIZE)
                .time_to_idle(CACHE_IDLE)
                .time_to_live(CACHE_MAX)
                .build();

            loop {
                let resp = client
                    .get(&url)
                    .basic_auth(&login, Some(&password))
                    .header(
                        "user-agent",
                        format!("{}/{}", crate_name!(), crate_version!()),
                    )
                    .header("content-type", "application/json")
                    .send();

                // Do not exit thread on server error, sleep and try to recover
                //
                let resp = match resp {
                    Ok(resp) => resp,
                    Err(e) => {
                        error!("worker-thread: {}", e.to_string());
                        stat_tx.send(StatMsg::Error).expect("stat::error");
                        thread::sleep(Duration::from_secs(2));
                        continue;
                    }
                };
                debug!("{:?}", &resp);

                // Check status of request.  We will ignore any error for now as the server
                // does not seem to be very stable.  It tends to returns 502 for transient errors.
                // So we sleep and continue
                //
                match resp.status() {
                    StatusCode::OK => {
                        trace!("OK");
                    }
                    code => {
                        let h = &resp.headers();
                        eprintln!("Error({}): {:?},", code, h);
                        stat_tx.send(StatMsg::Error).expect("stat::error");
                        thread::sleep(Duration::from_millis(stream_delay as u64));
                        continue;
                    }
                }

                let buf = resp.text().unwrap();

                // Retrieve answer and look into it, if answer was empty this should be rather fast
                //
                let sl: StateList = serde_json::from_str(buf.as_str()).expect("broken data");

                // Check whether data was returned
                //
                if sl.states.is_some() {
                    // Check whether we've seen it before
                    //
                    match cache.get(&sl.time) {
                        // We have seen it, loop
                        //
                        Some(_time) => {
                            eprint!("*");
                            stat_tx.send(StatMsg::Hits).expect("stat::hits");
                            thread::sleep(Duration::from_millis(stream_delay as u64));
                            continue;
                        }
                        // No, send it it and cache its `time`
                        //
                        _ => {
                            eprint!("{},", sl.time);

                            stat_tx.send(StatMsg::Miss).expect("stat::miss");
                            stat_tx.send(StatMsg::Pkts).expect("stat::pkts");
                            stat_tx
                                .send(StatMsg::Bytes(buf.len() as u64))
                                .expect("stat::bytes");

                            tx.send(buf).expect("send");
                            cache.insert(sl.time, true);
                        }
                    }
                } else {
                    // Are there still entries?  If no, then we have only empty traffic for CACHE_MAX.
                    //
                    stat_tx.send(StatMsg::Empty).expect("stat::empty");

                    cache.sync();
                    if cache.entry_count() == 0 {
                        eprintln!("No traffic, waiting for 2s.");
                        thread::sleep(Duration::from_secs(2_u64));
                    } else {
                        eprint!(".");
                    }
                }

                // Whatever happened, sleep for to avoid CPU/network overload
                if stream_delay != 0 {
                    thread::sleep(Duration::from_millis(stream_delay as u64));
                }
            }
        });

        // Now data gathering loop.  Should this be another thread?
        //
        loop {
            match rx.recv() {
                Ok(msg) => match msg.as_str() {
                    // Timer expired
                    //
                    "TIMEOUT" => {
                        trace!("End of scheduled run.");
                        break;
                    }
                    // Anything else is sent
                    //
                    _ => {
                        // Every record is separated with LF
                        //
                        out.send(format!("{}\n", msg))?;
                    }
                },
                _ => continue,
            }
        }
        // End threads
        //
        st_tx.send(StatMsg::Exit).unwrap();

        // sync; sync; sync
        //
        stat_id.join().unwrap();
        work_id.join().unwrap();
        disp_id.join().unwrap();

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
