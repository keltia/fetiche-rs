//! Avionix module.
//!
//! This module is for the Avionix Cube antenna API which supports only streams.
//!
//! There are one trait implementation:
//! - `Streamable`
//!

use chrono::Utc;
use clap::{crate_name, crate_version};
use mini_moka::sync::Cache;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{thread, time};
use serde_json::json;
use tracing::{debug, error, info, trace};

use crate::access::{StatMsg, Stats};
use crate::{Auth, AuthError, Capability, Filter, Site, Streamable};
use fetiche_formats::{Format, StateList};

const DEF_SITE: &str = "https://aero-network.com/api";


#[derive(Debug, Deserialize, Serialize)]
pub struct AvionixCube {
    /// Describe the different features of the source
    pub features: Vec<Capability>,
    /// Input formats
    pub format: Format,
    /// API Key
    pub api_key: String,
    /// User key
    pub user_key: String,
    /// API site
    pub base_url: String,
    /// Add this to `base_url` to fetch data
    pub get: String,
    /// reqwest blocking client
    pub client: Client,
    /// Running time (for streams)
    pub duration: i32,
}

impl AvionixCube {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("avionixcude::new");

        Self {
            ..Self::default()
        }
    }

    /// Load some data from in-memory loaded config
    ///
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("avionixcube::load");

        self.format = Format::from_str(&site.format).unwrap();
        self.base_url = site.base_url.to_owned();
        if let Some(auth) = &site.auth {
            match auth {
                Auth::UserKey {
                    api_key, user_key
                } => {
                    self.api_key = api_key.to_owned();
                    self.user_key = user_key.to_owned();
                }
                _ => {
                    error!("Bad auth parameter: {}", json!(auth));
                    panic!("nope");
                }
            }
        }
        self.get = site.route("stream").unwrap().to_owned();
        self
    }
}

impl Default for AvionixCube {
    fn default() -> Self {
        Self {
            features: vec![Capability::Stream],
            format: Format::AvionixCube,
            api_key: String::new(),
            user_key: String::new(),
            base_url: String::from(DEF_SITE),
            get: String::from("/json"),
            client: Client::new(),
            duration: 0,
        }
    }
}

impl Streamable for AvionixCube {
    fn format(&self) -> Format {
        Format::AvionixCube
    }

    fn name(&self) -> String {
        String::from("AvionixCube")
    }

    fn authenticate(&self) -> eyre::Result<String, AuthError> {
        trace!("fake token retrieval");
        Ok(format!("{}:{}", self.api_key, self.user_key))
    }

    /// The main stream function, inspired by Opensky one.
    ///
    /// We have a 5s window for drone movements so we need to poll every 5s, we cache all records
    /// during that 5s window to avoid dups.
    ///
    /// Right now it runs until killed by Ctrl+C or the timer expire (if set).
    ///
    /// The cache might be overkill because keeping only the last timestamp might be enough but:
    /// - it is easy to code and use
    /// - it helps to determine whether we had lack of traffic for a longer time if we have no
    ///   cached entries
    ///
    #[tracing::instrument(skip(self, out))]
    fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> eyre::Result<()> {
        trace!("avionixcube::stream");

    /// Max time we get data for
    const MAX_INTERVAL: Duration = Duration::from_secs(5);
    /// Expiration after insert/get
    const CACHE_IDLE: Duration = Duration::from_secs(10);
    /// Expiration after insert
    const CACHE_MAX: Duration = Duration::from_secs(30);
    /// Cache max entries
    const CACHE_SIZE: u64 = 20;



        let mut stream_duration = Duration::new(0, 0);
        let mut stream_interval = MAX_INTERVAL;

        let now = Utc::now().timestamp();

        trace!("avionixcube::stream(as {}:{})", self.api_key, self.user_key);

        let url = format!("{}{}", self.base_url, self.get);
        trace!("Streaming data from {}…", url);

        // FIXME: we can have only one argument
        //
        let args = Filter::from(args);
        let (min, max) = match args {
            Filter::Altitude {
                min,
                max,
            } => {
                (Some(min), Some(max))
            },
            _ => (None, None)
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
                thread::sleep(d);
                tx1.send("TIMEOUT".to_string()).unwrap();
            });
            trace!("end of sleep");
        }

        // reqwest::blocking::Client
        //
        let client = self.client.clone();

        let api_key = self.api_key.clone();
        let user_key = self.user_key.clone();

        // Launch stat gathering thread.
        //
        let (st_tx, st_rx) = channel::<StatMsg>();
        thread::spawn(move || {
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
                        break;
                    }
                }
            }
            eprintln!("\nSession: {}", stats);
            trace!("end of stats thread");
        });

        // Launch a thread that sleep for 30s then ask for statistics
        //
        let disp_tx = st_tx.clone();
        thread::spawn(move || {
            trace!("stats::display");
            loop {
                thread::sleep(Duration::from_secs(30_u64));
                let _ = disp_tx.send(StatMsg::Print);
            }
        });

        // Worker thread1
        //
        let stat_tx = st_tx.clone();
        thread::spawn(move || {
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
                    .header(
                        "user-agent",
                        format!("{}/{}", crate_name!(), crate_version!()),
                    )
                    .header("content-type", "application/json")
                    .header("api_key", &api_key)
                    .header("user_key", &user_key)
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
                            let _ = stat_tx.send(StatMsg::Hits);
                            thread::sleep(Duration::from_millis(stream_delay as u64));
                            continue;
                        }
                        // No, send it it and cache its `time`
                        //
                        _ => {
                            eprint!("{},", sl.time);

                            let _ = stat_tx.send(StatMsg::Miss);
                            let _ = stat_tx.send(StatMsg::Pkts);
                            let _ = stat_tx.send(StatMsg::Bytes(buf.len() as u64));

                            tx.send(buf).expect("send");
                            cache.insert(sl.time, true);
                        }
                    }
                } else {
                    // Are there still entries?  If no, then we have only empty traffic for CACHE_MAX.
                    //
                    let _ = stat_tx.send(StatMsg::Empty);

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
        let _ = st_tx.send(StatMsg::Exit);

        // sync; sync; sync
        //
        Ok(())
    }



        todo!()
    }
}
