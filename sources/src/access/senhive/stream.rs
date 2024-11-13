use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use eyre::Result;
use lapin::{Connection, ConnectionProperties};
use ractor::Actor;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use tokio::time::sleep;
use tracing::{info, trace};

use fetiche_formats::Format;

use crate::access::senhive::StatMsg;
use crate::actors::{StatOps, StatsActor};
use crate::Stats;
use crate::{AsyncStreamable, AuthError, Filter, Senhive};

const TICK: Duration = Duration::from_secs(30);

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
            Filter::Duration(duration) => Duration::from_secs(duration as u64),
            _ => Duration::new(0, 0),
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

        let (tx, rx) = channel::<String>();

        // Start the stats gathering actor.
        //
        trace!("starting stats actor.");
        let tag = String::from("senhive::stream");
        let (w, h) = Actor::spawn(Some(tag), StatsActor, ()).await?;

        // Every TICK, we display stats.
        //
        w.send_interval(TICK, || StatOps::Print).await?;

        // Set the clock ticking unless duration is 0
        //
        if stream_duration != Duration::from_secs(0) {
            w.exit_after(stream_duration).await?;
        }

        // Worker thread1
        //
        let url = self.base_url.clone();

        // We have to use an async thread. Actor, anyone?
        //
        tokio::spawn(async move {
            trace!("Starting worker thread");

            // Do the connection
            //
            trace!("tcp::connect");
            let conn = Connection::connect(&url, ConnectionProperties::default())
                .await
                .expect("connect failed");
        });

        // End threads
        //
        let _ = w.stop("end".into()).await?;

        Ok(())
    }

    fn format(&self) -> Format {
        Format::Senhive
    }
}
