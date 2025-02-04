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

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use ractor::pg::join;
use ractor::{call, pg, Actor, ActorRef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, info, trace, warn};

use super::actors::{Worker, WorkerArgs};
use crate::actors::{StatsMsg, Supervisor};
use crate::{Auth, AuthError, Capability, Filter, Routes, Site, Stats, StatsError, Streamable, StreamableSource, WorkerMsg, ENGINE_PG};
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
    /// Through the get route, we middle traffic
    pub src: String,
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

        // the "get" route is used to filter RID vs A sources
        //
        let routes = site.routes.clone().unwrap_or_else(|| {
            // If no routes have been defined, assume we want only RID
            //
            warn!("No routes defined for AvionixServer");

            let mut bt = BTreeMap::new();
            bt.insert("get".to_string(), "RID".to_string());
            Routes::from(bt.clone())
        });
        self.src = routes.get("get").unwrap().to_owned();

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

    pub fn source(&self) -> StreamableSource {
        StreamableSource::AvionixServer(self.clone())
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
            src: String::from("RID"),
        }
    }
}

#[async_trait]
impl Streamable for AvionixServer {
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
            traffic: self.src.clone(),
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
            ENGINE_PG.into(),
            vec![sup.get_cell(), worker.get_cell(), stat.get_cell()],
        );

        info!("List of actors.");
        let list = pg::get_members(&ENGINE_PG.to_string());
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
            let (_tx, rx) = std::sync::mpsc::channel::<()>();
            rx.recv().expect("Something failed here.");
        }

        // End threads
        //
        trace!("avionixserver::stream stopping.");

        // Stop everyone in the group.
        //
        pg::get_members(&ENGINE_PG.to_string())
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
