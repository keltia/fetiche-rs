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
//! We implement the 2nd one as it is simpler and does not need any cache.
//!
//! NOTE: the flow includes several kind of data, drones and airplanes.
//!

use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use ractor::pg::join;
use ractor::{pg, Actor};
use serde::{Deserialize, Serialize};
use serde_json::json;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use tracing::{error, info, trace};

use super::actors::{Worker, WorkerArgs};
use crate::access::TICK;
use crate::actors::{StatsActor, StatsMsg, Supervisor, PG_SOURCES};
use crate::{AsyncStreamable, Auth, AuthError, Capability, Filter, Site, WorkerMsg};
use fetiche_formats::Format;

/// TCP streaming URL
pub(crate) const DEF_SITE: &str = "tcp.aero-network.com";
/// TCP streaming port
pub(crate) const DEF_PORT: u16 = 50007;

#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[async_trait]
impl AsyncStreamable for AvionixServer {
    fn name(&self) -> String {
        String::from("AvionixServer")
    }

    #[tracing::instrument(skip(self))]
    async fn authenticate(&self) -> eyre::Result<String, AuthError> {
        trace!("fake token retrieval");
        Ok(String::from(""))
    }

    /// The main stream function, inspired by Opensky one.
    ///
    /// Right now it runs until killed by Ctrl+C or the timer expire (if set).
    ///
    ///
    #[tracing::instrument(skip(self, out))]
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> eyre::Result<()> {
        trace!("avionixserver::stream");

        let filter = Filter::from(args);

        let stream_duration = match filter {
            Filter::Altitude { duration, .. } => Duration::from_secs(duration as u64),
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

        // We have a generic supervisor actor.
        //
        trace!("starting supervisor actor.");
        let tag = String::from("avionixserver::supervisor");
        let (sup, _h) = Actor::spawn(Some(tag), Supervisor, ()).await?;

        // Start the stats gathering actor.
        //
        trace!("starting stats actor.");
        let tag = String::from("avionix::stats");
        let (stat, _h) = Actor::spawn_linked(
            Some(tag),
            StatsActor,
            "avionixserver".into(),
            sup.get_cell(),
        )
            .await?;

        // Launch the worker actor
        //
        let url = format!(
            "tcp://{}:{}@{}",
            self.api_key,
            self.user_key,
            self.base_url.clone()
        );
        trace!("Starting worker actor.");
        let args = WorkerArgs {
            url,
            out,
            stat: stat.clone(),
        };
        let tag = String::from("avionixserver::worker");
        let (worker, _handle) =
            Actor::spawn_linked(Some(tag), Worker, args, sup.get_cell()).await?;

        // Every TICK, we display stats.
        //
        stat.send_interval(TICK, || StatsMsg::Print);

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

        // Get the ball rolling.
        //
        let _ = worker.cast(WorkerMsg::Consume(filter, stream_duration.as_secs()))?;

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
        trace!("avionixserver::stream stopping.");

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
        Format::CubeData
    }
}
