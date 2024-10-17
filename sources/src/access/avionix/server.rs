//! Avionix Server module.
//!
//! This module is for the Avionix Cube antenna API which supports only streams.
//!
//! There are one trait implementation:
//! - `Streamable`
//!
//! There are two options here:
//! - HTTP call on usual TLS port, not more than 1 call/s with a 5s window
//! - streaming JSONL records by connecting to port 50007
//!
//! We implement the 2nd one as it is simpler and does not need any cache..
//!

use std::io::{Cursor, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use polars::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use tracing::{debug, error, info, trace};

use crate::access::avionix::BUFSIZ;
use crate::access::Stats;
use crate::{Auth, AuthError, Capability, Filter, Site, Streamable};
use fetiche_formats::Format;

/// TCP streaming URL
const DEF_SITE: &str = "tcp.aero-network.com";
/// TCP streaming port
const DEF_PORT: u16 = 50007;

/// Messages to send to the stats threads
///
#[derive(Clone, Debug, Serialize)]
enum StatMsg {
    Pkts(u32),
    Bytes(u64),
    Empty,
    Error,
    Print,
    Exit,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AvionixServer {
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
    /// Running time (for streams)
    pub duration: i32,
}

impl AvionixServer {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("avionixserver::new");

        Self { ..Self::default() }
    }

    /// Load some data from in-memory loaded config
    ///
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("avionixserver::load");

        self.format = Format::from_str(&site.format).unwrap();
        self.base_url = site.base_url.to_owned();
        if let Some(auth) = &site.auth {
            match auth {
                Auth::UserKey { api_key, user_key } => {
                    self.api_key = api_key.to_owned();
                    self.user_key = user_key.to_owned();
                }
                _ => {
                    error!("Bad auth parameter: {}", json!(auth));
                    panic!("nope");
                }
            }
        }
        self
    }
}

impl Default for AvionixServer {
    fn default() -> Self {
        Self {
            features: vec![Capability::Stream],
            format: Format::CubeData,
            api_key: String::new(),
            user_key: String::new(),
            base_url: String::from(DEF_SITE),
            duration: 0,
        }
    }
}

impl Streamable for AvionixServer {
    fn name(&self) -> String {
        String::from("AvionixServer")
    }

    fn authenticate(&self) -> eyre::Result<String, AuthError> {
        trace!("fake token retrieval");
        Ok(String::from(""))
    }

    /// The main stream function, inspired by Opensky one.
    ///
    /// Right now it runs until killed by Ctrl+C or the timer expire (if set).
    ///
    ///
    #[tracing::instrument(skip(self, out))]
    fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> eyre::Result<()> {
        trace!("avionixcube::stream");

        /// Stats loop
        const STATS_LOOP: Duration = Duration::from_secs(30);
        const START_MARKER: &str = "\x02";

        let stream_duration = Duration::new(0, 0);

        trace!("Streaming data from {}…", self.base_url);

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
        if stream_duration != Duration::from_secs(0) {
            trace!("setup wakeup alarm");

            let d = stream_duration;
            let tx1 = tx.clone();
            thread::spawn(move || {
                trace!("alarm set to {}s", d.as_secs());
                thread::sleep(d);
                tx1.send("TIMEOUT".into()).unwrap();
            });
            trace!("end of sleep");
        }

        // Launch stat gathering thread.
        //
        let (st_tx, st_rx) = channel::<StatMsg>();
        thread::spawn(move || {
            trace!("stats::thread");

            let start = Instant::now();
            let mut stats = Stats::default();
            while let Ok(msg) = st_rx.recv() {
                match msg {
                    StatMsg::Pkts(n) => stats.pkts += n,
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
                thread::sleep(STATS_LOOP);
                let _ = disp_tx.send(StatMsg::Print);
            }
        });

        // Worker thread1
        //
        let stat_tx = st_tx.clone();
        let args = args.clone();
        let url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let user_key = self.user_key.clone();
        thread::spawn(move || {
            trace!("Starting worker thread");

            // Do the connection
            //
            let mut conn = TcpStream::connect(url).expect("connect failed");

            // Send credentials
            //
            let auth_str = format!("{}\n{}\n", self.api_key, self.user_key);
            conn.write(auth_str.as_bytes()).expect("auth write failed");

            trace!("avionixcube::stream(as {}:{})", api_key, user_key);

            // FIXME: we can have only one argument
            //
            let args = Filter::from(args);
            let (min, max) = match args {
                Filter::Altitude { min, max } => (Some(min), Some(max)),
                _ => (None, None),
            };

            // Manage url parameters.  Assume that if one is defined, the other is as well.
            //
            if min.is_some() {
                let min = min.unwrap();
                let min_str = format!("min_altitude={min}\n");
                conn.write(min_str.as_bytes()).expect("write failed");
            }
            if max.is_some() {
                let max = max.unwrap();
                let max_str = format!("max_altitude={max}\n");
                conn.write(max_str.as_bytes()).expect("write failed");
            };

            info!(
            r##"
StreamURL: {}
Duration {}s

<number>: data packet / ".": no traffic / "*": cache hit
        "##,
                url,
                stream_duration.as_secs()
            );

            let mut buf = String::with_capacity(BUFSIZ);

            // Start stream
            //
            conn.write(START_MARKER.as_ref()).expect("failed to write marker");
            conn.flush().expect("flush marker");
            loop {
                match conn.read(&mut buf.as_ref()) {
                    Ok(size) => {
                        trace!("{} bytes read.", size);
                    }
                    Err(e) => {
                        error!("worker-thread: {}", e.to_string());
                        stat_tx.send(StatMsg::Error).expect("stat::error");

                        conn.shutdown(Shutdown::Both).expect("shutdown socket");

                        // Do the connection again
                        //
                        conn = TcpStream::connect(&self.base_url).expect("connect socket");
                        continue;
                    }
                }
                let cur = Cursor::new(buf.as_bytes());
                let df = JsonLineReader::new(cur).finish().expect("create dataframe");
                debug!("{:?}", df);

                let _ = stat_tx.send(StatMsg::Pkts(df.iter().len() as u32));
                let _ = stat_tx.send(StatMsg::Bytes(buf.len() as u64));

                tx.send(buf).expect("send");
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

    fn format(&self) -> Format {
        Format::CubeData
    }
}
