//! Module that implement the `AsyncStreamable` trait.
//!

use std::sync::mpsc::Sender;
use std::time::Duration;

use async_trait::async_trait;
use eyre::Result;
use ractor::pg::join;
use ractor::Actor;
use tracing::trace;

use fetiche_formats::Format;

use super::actors::{Worker, WorkerMsg, WorkerState};
use crate::actors::{StatsActor, StatsMsg, PG_SOURCES};
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
    /// We start by draining the `dl_fused_data` queue, then switch to the regular `fused_data` one
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

        // Start the stats gathering actor.
        //
        trace!("starting stats actor.");
        let tag = String::from("senhive::stats");
        let (stat, _h) = Actor::spawn(Some(tag), StatsActor, ()).await?;

        // Every TICK, we display stats.
        //
        stat.send_interval(TICK, || StatsMsg::Print).await?;

        // Set the clock ticking unless duration is 0
        //
        if stream_duration != Duration::from_secs(0) {
            stat.exit_after(stream_duration).await?;
        }

        // Launch the worker actor
        //
        trace!("Starting worker actor.");
        let args = WorkerState::new(out.clone(), stat.clone());
        let tag = String::from("senhive::worker");
        let (worker, _handle) = Actor::spawn(Some(tag), Worker, args).await?;

        // Start the processing.
        //
        let url = self.base_url.clone();
        let _ = worker.cast(WorkerMsg::Start(url))?;

        // End threads
        //
        trace!("Senhive::stream stopping.");

        let _ = stat.stop(Some("end".into()));
        let _ = worker.stop(None);

        join(PG_SOURCES.into(), vec![worker.get_cell(), stat.get_cell()]);

        Ok(())
    }

    fn format(&self) -> Format {
        Format::Senhive
    }
}

