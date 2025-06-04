//! Module that implements the `Streamable` trait for Senhive data sources.
//!
//! This module provides streaming functionality for Senhive data sources using an actor-based
//! architecture. The implementation uses the following components:
//!
//! - A Worker actor that handles the AMQP queue consumption
//! - A Stats actor that collects and reports streaming statistics
//! - Process groups to manage actor lifecycle
//!
//! The streaming can be configured to run for a specific duration or indefinitely until
//! interrupted. The implementation handles:
//!
//! - Actor setup and teardown
//! - Stream duration management
//! - Statistics collection
//! - Process group membership
//!
//! The module interfaces with AMQP queues to receive both stored and current data from
//! the Senhive data source.
//!
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

use eyre::Result;
use ractor::{call, pg, Actor, ActorRef};
use tracing::{info, trace};

use fetiche_formats::Format;

use super::actors::{Worker, WorkerArgs, WorkerMsg};
use crate::actors::StatsMsg;
use crate::sources::SENHIVE_PG;
use crate::{AuthError, Filter, Senhive, Stats, StatsError, Streamable};

impl Senhive {
    async fn setup_actors(
        &self,
        out: Sender<String>,
        stat: ActorRef<StatsMsg>,
    ) -> Result<(ActorRef<WorkerMsg>, String)> {
        let url = self.base_url.clone();
        trace!("Starting worker actor.");

        let args = WorkerArgs { url, out, stat: stat.clone() };
        let tag = String::from("senhive::worker");
        let (worker, _handle) = Actor::spawn(Some(tag.clone()), Worker, args).await?;

        pg::join(SENHIVE_PG.into(), vec![worker.get_cell(), stat.get_cell()]);

        info!("List of actors in PG.");
        pg::get_members(&SENHIVE_PG.to_string())
            .iter()
            .for_each(|member| {
                info!("  {}", member.get_name().unwrap_or("<anon>".into()));
            });

        Ok((worker, tag))
    }
}

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
    #[tracing::instrument(skip(self, out, _token, args))]
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<Stats> {
        let args = Filter::from(args);
        let stream_duration = match args {
            Filter::Stream { duration, .. } => Duration::from_secs(duration as u64),
            _ => Duration::new(0, 0),
        };
        trace!(
            "Streaming data from {} for {}s",
            self.base_url,
            stream_duration.as_secs()
        );

        let stat = self.stat.clone().ok_or(StatsError::NotInitialized)?;
        let (worker, tag) = self.setup_actors(out, stat.clone()).await?;

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
