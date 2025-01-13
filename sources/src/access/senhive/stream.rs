//! Module that implement the `AsyncStreamable` trait.
//!

use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

use async_trait::async_trait;
use eyre::Result;
use ractor::pg::join;
use ractor::{pg, Actor};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;
use tracing::{info, trace};

use fetiche_formats::Format;

use super::actors::{Worker, WorkerArgs, WorkerMsg};
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
            Filter::Stream { duration, .. } => Duration::from_secs(duration as u64),
            _ => Duration::new(0, 0),
        };
        trace!(
            "Streaming data from {} for {}s",
            self.base_url,
            stream_duration.as_secs()
        );

        // setup ctrl-c handled
        //
        #[cfg(windows)]
        let mut sig = ctrl_c().unwrap();

        #[cfg(unix)]
        let mut stream = signal(SignalKind::interrupt()).unwrap();

        // We have a generic supervisor actor.
        //
        trace!("starting supervisor actor.");
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
        let (worker, _handle) =
            Actor::spawn_linked(Some(tag), Worker, args, sup.get_cell()).await?;

        // Insert each actor in the PG_SOURCES group.
        //
        join(
            PG_SOURCES.into(),
            vec![sup.get_cell(), worker.get_cell(), stat.get_cell()],
        );

        info!("List of actors.");
        let list = pg::get_members(&PG_SOURCES.to_string());
        list.iter().for_each(|member| {
            info!("  {}", member.get_name().unwrap_or("<anon>".into()));
        });

        // Every TICK, we display stats.
        //
        stat.send_interval(TICK, || StatsMsg::Print);

        // Setup signal handling.
        //
        tokio::spawn(async move {
            trace!("SIGINT thread running.");

            // Wait for completion or interrupt
            //
            #[cfg(unix)]
            if let Some(_) = stream.recv().await {
                info!("Got SIGINT.");
                break;
            }
            #[cfg(windows)]
            sig.recv().await;

            info!("^C pressed.");

            // Stop everyone in the group.
            //
            pg::get_members(&PG_SOURCES.to_string())
                .iter()
                .for_each(|member| {
                    member.stop(Some("^C ^pressed, ending.".to_string()));
                });
            std::process::exit(0);
        });

        // Start the processing.
        //
        let _ = worker.cast(WorkerMsg::Consume("fused_data".into(), "data".into()))?;

        // Set the clock ticking unless duration is 0
        //
        info!("Get clock ticking.");
        if stream_duration != Duration::from_secs(0) {
            info!("Sleeping for {}s.", stream_duration.as_secs());
            worker.exit_after(stream_duration);
            tokio::time::sleep(stream_duration).await;
            info!("Timer expired.");
        } else {
            // We somehow needs to wait for a ^C.
            //
            let (_tx, rx) = channel::<()>();
            rx.recv().expect("Something failed here.");
        }

        // End threads
        //
        trace!("Senhive::stream stopping.");

        // Stop everyone in the group.
        //
        pg::get_members(&PG_SOURCES.to_string())
            .iter()
            .for_each(|member| {
                member.stop(Some("Ending.".to_string()));
            });

        Ok(())
    }

    fn format(&self) -> Format {
        Format::Senhive
    }
}
