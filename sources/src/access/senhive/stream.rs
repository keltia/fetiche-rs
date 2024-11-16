//! Module that implement the `AsyncStreamable` trait.
//!

use std::sync::mpsc::Sender;
use std::time::Duration;

use async_trait::async_trait;
use eyre::Result;
use ractor::pg::join;
use ractor::rpc::{call, cast};
use ractor::time::{exit_after, send_interval};
use ractor::{call, cast, pg, Actor};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;
use tracing::{info, trace};

use fetiche_formats::Format;

use super::actors::{Worker, WorkerArgs, WorkerMsg, WorkerState};
use crate::actors::{StatsActor, StatsMsg, Supervisor, PG_SOURCES};
use crate::{AsyncStreamable, AuthError, Filter, Senhive};

const TICK: Duration = Duration::from_secs(30);

#[async_trait]
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
    /// We now use different actors (see [actor.rs]) to handle the manage the different
    /// topics/queues.
    ///
    /// - `Supervisor` to manage the worker and stats actors
    /// - `StatsActor` is sent a `Tick` message every 30s to display stats and called on each packet
    ///   to accumulate stats
    /// - `Worker` is then launched to read `fused_data` and its dead letter equivalent to
    ///   get both stored and current data.
    ///
    ///
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<()> {
        trace!("Senhive::stream");

        let args = Filter::from(args);

        // 0 means forever.
        //
        let stream_duration = match args {
            Filter::Duration(duration) => Duration::from_secs(duration as u64),
            _ => Duration::new(0, 0),
        };

        trace!("Streaming data from {}…", self.base_url);

        // setup ctrl-c handled
        //
        #[cfg(windows)]
        let mut sig = ctrl_c().unwrap();

        #[cfg(unix)]
        let mut stream = signal(SignalKind::interrupt()).unwrap();

        // We have a generic supervisor actor.
        //
        let tag = String::from("senhive:supervisor");
        let (sup, _h) = Actor::spawn(Some(tag), Supervisor, ()).await?;

        // Start the stats gathering actor.
        //
        trace!("starting stats actor.");
        let tag = String::from("senhive::stats");
        let (stat, _h) =
            Actor::spawn_linked(Some(tag), StatsActor, "senhive".into(), sup.get_cell()).await?;

        // Launch the worker actor
        //
        let url = self.base_url.clone();
        trace!("Starting worker actor.");
        let args = WorkerArgs {
            url,
            out,
            stat: stat.clone(),
        };
        let tag = String::from("senhive::worker");
        let (worker, handle) = Actor::spawn_linked(Some(tag), Worker, args, sup.get_cell()).await?;

        // Insert each actor in the PG_SOURCES group.
        //
        join(PG_SOURCES.into(), vec![worker.get_cell(), stat.get_cell()]);

        info!("List of actors.");
        let list = pg::get_members(&PG_SOURCES.to_string());
        list.iter().for_each(|member| {
            info!("  {}", member.get_name().unwrap_or("<anon>".into()));
        });

        // Every TICK, we display stats.
        //
        let _ = send_interval(TICK, stat.get_cell(), || StatsMsg::Print);

        // Start the processing.
        //
        let url = self.base_url.clone();
        let _ = cast(
            &worker.get_cell(),
            WorkerMsg::Consume("fused_data".into(), "data".into()),
        )?;

        // Set the clock ticking unless duration is 0
        //
        info!("Get clock ticking.");
        if stream_duration != Duration::from_secs(0) {
            let _ = exit_after(stream_duration, worker.get_cell());
            tokio::time::sleep(stream_duration).await;
        } else {
            // Wait for completion or interrupt
            //
            #[cfg(unix)]
            if let Some(_) = stream.recv() {
                info!("Got SIGINT.");
            }

            #[cfg(windows)]
            if (sig.recv().await).is_some() {
                info!("^C pressed.");
            }
        }
        // End threads
        //
        trace!("Senhive::stream stopping.");

        let _ = stat.stop(Some("end".into()));
        let _ = worker.stop(None);

        Ok(())
    }

    fn format(&self) -> Format {
        Format::Senhive
    }
}
