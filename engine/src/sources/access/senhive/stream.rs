//! Module that implement the `Streamable` trait.
//!

use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

use eyre::Result;
use ractor::pg::join;
use ractor::{call, pg, Actor};
use tracing::{info, trace};

use fetiche_formats::Format;

use super::actors::{Worker, WorkerArgs, WorkerMsg};
use crate::actors::StatsMsg;
use crate::sources::SENHIVE_PG;
use crate::{AuthError, Filter, Senhive, Stats, StatsError, Streamable};

impl Streamable for Senhive {
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
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<Stats> {
        trace!("Senhive::stream");

        let args = Filter::from(args);
        let stat = match self.stat.clone() {
            Some(stat) => stat,
            None => return Err(StatsError::NotInitialized.into())
        };

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
            Actor::spawn(Some(tag.clone()), Worker, args).await?;

        // Insert each actor in the PG_SOURCES group.
        //
        join(
            SENHIVE_PG.into(),
            vec![worker.get_cell(), stat.get_cell()],
        );

        info!("List of actors.");
        let list = pg::get_members(&SENHIVE_PG.to_string());
        list.iter().for_each(|member| {
            info!("  {}", member.get_name().unwrap_or("<anon>".into()));
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

        let stats = call!(stat, |port| StatsMsg::Exit(tag, port))?;
        // Stop everyone in the group.
        //
        pg::get_members(&SENHIVE_PG.to_string())
            .iter()
            .for_each(|member| {
                member.stop(Some("Ending.".to_string()));
            });

        Ok(stats)
    }

    fn format(&self) -> Format {
        Format::Senhive
    }
}
