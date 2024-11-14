//! Avionix Cube module.
//!
//! This module is for the Avionix Cube antenna direct access which means reduced filters and no auth.
//!
//! There are one trait implementation:
//! - `Streamable`
//!
//! TCP Streaming on port 50005
//!
//! NOTE: the flow includes several kind of data, drones and airplanes.
//!

use std::io::{BufReader, Cursor, Read};
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use polars::prelude::*;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use tracing::{debug, error, info, trace};

use crate::access::avionix::BUFSIZ;
use crate::actors::StatsMsg;
use crate::{Auth, AuthError, Capability, Filter, Site, Stats, Streamable};
use fetiche_formats::Format;

/// TCP streaming port
const DEF_PORT: u16 = 50005;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AvionixCube {
    /// Describe the different features of the source
    pub features: Vec<Capability>,
    /// Input formats
    pub format: Format,
    /// API site
    pub base_url: String,
    /// Running time (for streams)
    pub duration: i32,
}

impl AvionixCube {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("avionixcube::new");

        Self { ..Self::default() }
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
                Auth::Anon => {}
                _ => {
                    error!("Bad auth parameter: {}", json!(auth));
                    panic!("nope");
                }
            }
        }
        self
    }
}

impl Default for AvionixCube {
    fn default() -> Self {
        Self {
            features: vec![Capability::Stream],
            format: Format::CubeData,
            base_url: String::from("CHANGEME"),
            duration: 0,
        }
    }
}

impl Streamable for AvionixCube {
    fn name(&self) -> String {
        String::from("AvionixCube")
    }

    fn authenticate(&self) -> eyre::Result<String, AuthError> {
        trace!("fake token retrieval");
        Ok(String::from(""))
    }

    /// The main stream function, inspired by Opensky one.
    ///
    /// No cache is needed because it is plain TCP streaming.
    ///
    #[tracing::instrument(skip(self, out))]
    fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> eyre::Result<()> {
        trace!("avionixcube::stream");

        /// Stats loop
        const STATS_LOOP: Duration = Duration::from_secs(30);

        let args = Filter::from(args);

        let stream_duration = match args {
            Filter::Altitude { duration, .. } => Duration::from_secs(duration as u64),
            _ => Duration::new(0, 0),
        };

        trace!("Streaming data from {}…", self.base_url);

        info!(
            r##"
StreamURL: {}
Duration {}s
        "##,
            self.base_url,
            stream_duration.as_secs()
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
        if stream_duration != Duration::from_secs(0) {
            trace!("setup wakeup alarm");

            let d = stream_duration;
            let tx1 = tx.clone();
            thread::spawn(move || {
                trace!("alarm set to {}s", d.as_secs());
                thread::sleep(d);
                tx1.send("TIMEOUT".to_string()).unwrap();
            });
            trace!("end of sleep");
        }

        // Launch stat gathering thread.
        //
        let (st_tx, st_rx) = channel::<StatsMsg>();
        thread::spawn(move || {
            trace!("stats::thread");

            let start = Instant::now();
            let mut stats = Stats::default();
            while let Ok(msg) = st_rx.recv() {
                match msg {
                    StatsMsg::Pkts(n) => stats.pkts += n,
                    StatsMsg::Error => stats.err += 1,
                    StatsMsg::Reconnect => stats.reconnect += 1,
                    StatsMsg::Bytes(n) => stats.bytes += n,
                    StatsMsg::Print => {
                        stats.tm = start.elapsed().as_secs();
                        eprintln!("Stats: {}", stats)
                    }
                    StatsMsg::Reset => (),
                    // The end
                    StatsMsg::Exit => {
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
                let _ = disp_tx.send(StatsMsg::Print);
            }
        });

        let url = self.base_url.clone();
        // if url has no port, add it
        //
        let url = match Url::from_str(&url)?.port() {
            Some(_) => url,
            None => format!("{}:{}", url, DEF_PORT),
        };

        // Worker thread1
        //
        let stat_tx = st_tx.clone();
        thread::spawn(move || {
            trace!("Starting worker thread");

            // Start stream
            //
            let mut conn_wt = BufReader::new(TcpStream::connect(&url).expect("connect socket"));

            loop {
                let mut buf = [0u8; BUFSIZ];

                match conn_wt.read(&mut buf) {
                    Ok(size) => {
                        trace!("{} bytes read.", size);
                    }
                    Err(e) => {
                        error!("worker-thread: {}", e.to_string());
                        stat_tx.send(StatsMsg::Error).expect("stat::error");

                        // Do the connection again
                        //
                        stat_tx.send(StatsMsg::Reconnect).expect("stat::exit");
                        conn_wt = BufReader::new(TcpStream::connect(&url).expect("connect socket"));
                        continue;
                    }
                }
                debug!("buf={buf:?}");

                let cur = Cursor::new(&buf);
                let df = JsonLineReader::new(cur).finish().unwrap();
                debug!("{:?}", df);

                let _ = stat_tx.send(StatsMsg::Pkts(df.iter().len() as u32));
                let _ = stat_tx.send(StatsMsg::Bytes(buf.as_ref().len() as u64));

                tx.send(String::from_utf8(buf.to_vec()).unwrap())
                    .expect("send");
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
                        out.send(format!("{}\n", msg)).expect("send data");
                    }
                },
                _ => continue,
            }
        }
        // End threads
        //
        let _ = st_tx.send(StatsMsg::Exit);

        // sync; sync; sync
        //
        Ok(())
    }

    fn format(&self) -> Format {
        self.format
    }
}
