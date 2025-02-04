use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{thread, time};

use chrono::Utc;
use clap::{crate_name, crate_version};
use mini_moka::sync::Cache;
use reqwest::StatusCode;
use tracing::{debug, error, info, trace};

use crate::{AuthError, Filter, Stats, Streamable};
use fetiche_formats::Format;

impl Streamable for Opensky {
    fn name(&self) -> String {
        "opensky".to_string()
    }

    /// All credentials are passed every time we call the API so return a fake token
    ///
    fn authenticate(&self) -> Result<String, AuthError> {
        trace!("fake token retrieval");
        Ok("".into())
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
    /// - it helps to determine whether we had lack of traffic for a longer time if we have no
    ///   cached entries
    ///
    #[tracing::instrument(skip(self, out))]
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<()> {
        trace!("opensky::stream");

        let mut stream_duration = 0;
        let mut stream_delay = 1000;

        let now = Utc::now().timestamp();

        let login = self.login.clone();
        let password = self.password.clone();
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
                    let start = if now - from > crate::access::opensky::MAX_INTERVAL {
                        now - crate::access::opensky::MAX_INTERVAL
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
                        stat_tx
                            .send(StatMsg::Error)
                            .expect("stat::error");
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
                        stat_tx
                            .send(StatMsg::Error)
                            .expect("stat::error");
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
                            let _ = stat_tx.send(crate::access::opensky::StatMsg::Hits);
                            thread::sleep(Duration::from_millis(stream_delay as u64));
                            continue;
                        }
                        // No, send it it and cache its `time`
                        //
                        _ => {
                            eprint!("{},", sl.time);

                            let _ = stat_tx.send(StatMsg::Miss);
                            let _ = stat_tx.send(StatMsg::Pkts);
                            let _ = st
                                .send(StatMsg::Bytes(buf.len() as u64));

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

    fn format(&self) -> Format {
        Format::Opensky
    }
}
