use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use eyre::Result;
use lapin::{Connection, ConnectionProperties};
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use tokio::sync::mpsc::{channel, Sender};
use tokio::time::sleep;
use tracing::{info, trace};

use fetiche_formats::Format;

use crate::access::senhive::StatMsg;
use crate::access::Stats;
use crate::{AsyncStreamable, AuthError, Filter, Senhive};

impl AsyncStreamable for Senhive {
    fn name(&self) -> String {
        String::from("Senhive")
    }

    async fn authenticate(&self) -> Result<String, AuthError> {
        trace!("Senhive::authenticate, fake token");

        Ok(String::from(""))
    }

    /// Set up streaming from the AMQP queues.
    ///
    /// We start by draining the `dl_fused_data` queue, then switch to the regular `fused_data` one
    ///
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<()> {
        trace!("Senhive::stream");

        // Stats loop
        const STATS_LOOP: Duration = Duration::from_secs(30);

        let args = Filter::from(args);

        // 0 means forever.
        //
        let stream_duration = match args {
            Filter::Duration(duration) => { Duration::from_secs(duration as u64) }
            _ => Duration::new(0, 0)
        };

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

        let (tx, rx) = channel::<String>(10);

        // Timer set?  If yes, launch a sleeper thread
        //
        if stream_duration != Duration::from_secs(0) {
            trace!("setup wakeup alarm");

            let d = stream_duration;
            let tx1 = tx.clone();
            tokio::spawn(async move {
                trace!("alarm set to {}s", d.as_secs());
                sleep(d).await;
                info!("DING for {}", d.as_secs());
                tx1.send("TIMEOUT".into()).unwrap();
            });
            trace!("end of sleep");
        }

        // Launch stat gathering thread.
        //
        let (st_tx, st_rx) = channel::<StatMsg>();
        tokio::spawn(async move {
            trace!("stats::thread");

            let start = Instant::now();
            let mut stats = Stats::default();
            while let Ok(msg) = st_rx.recv() {
                match msg {
                    StatMsg::Pkts(n) => stats.pkts += n,
                    StatMsg::Empty => stats.empty += 1,
                    StatMsg::Error => stats.err += 1,
                    StatMsg::Reconnect => stats.reconnect += 1,
                    StatMsg::Bytes(n) => stats.bytes += n,
                    StatMsg::Print => {
                        stats.tm = start.elapsed().as_secs();
                        info!("Stats: {}", stats)
                    }
                    // The end
                    StatMsg::Exit => {
                        stats.tm = start.elapsed().as_secs();
                        break;
                    }
                }
            }
            info!("\nSession: {}", stats);
            trace!("end of stats thread");
        });

        // Launch a thread that sleep for 30s then ask for statistics
        //
        let disp_tx = st_tx.clone();
        tokio::spawn(async move {
            trace!("stats::display");
            loop {
                sleep(STATS_LOOP).await;
                trace!("TICK");
                let _ = disp_tx.send(StatMsg::Print);
            }
        });

        // Worker thread1
        //
        let stat_tx = st_tx.clone();
        let url = self.base_url.clone();

        // We have to use an async thread. Actor, anyone?
        //
        tokio::spawn(async move {
            trace!("Starting worker thread");

            // Do the connection
            //
            trace!("tcp::connect");
            let conn = Connection::connect(&url, ConnectionProperties::default()).await
                .expect("connect failed");
        });

        // End threads
        //
        let _ = st_tx.send(StatMsg::Exit);

        Ok(())
    }

    fn format(&self) -> Format {
        Format::Senhive
    }
}
