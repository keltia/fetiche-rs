//! Avionix Cube module.
//!
//! This module is for the Avionix Cube antenna direct access which means reduced filters and no auth.
//!
//! There are one trait implementation:
//! - `AsyncStreamable`
//!
//! TCP Streaming on port 50005
//!
//! NOTE: the flow includes several kind of data, drones and airplanes.
//!

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::Duration;

use ractor::pg::join;
use ractor::{pg, Actor};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use tracing::{error, info, trace, warn};

use crate::access::TICK;
use crate::{AsyncStreamable, Auth, AuthError, Capability, Filter, LocalWorker, Routes, Site, StreamableSource, WorkerArgs, WorkerMsg};
use fetiche_formats::Format;

/// TCP streaming port
const DEF_PORT: u16 = 50005;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Cube {
    /// Describe the different features of the source
    pub feature: Capability,
    /// Input formats
    pub format: Format,
    /// Local IP of the antenna
    pub base_url: String,
    /// Running time (for streams)
    pub duration: i32,
    /// Filter the source
    pub src: String,
}

impl Cube {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("avionixcube::new");

        Self { ..Self::default() }
    }

    /// Load some data from in-memory loaded config
    ///
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("avionixcube::load");

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
                Auth::Anon => {}
                _ => {
                    error!("Bad auth parameter: {}", json!(auth));
                    panic!("nope");
                }
            }
        }
        self
    }

    pub fn source(&self) -> StreamableSource {
        StreamableSource::Cube(self.clone())
    }
}

impl Default for Cube {
    fn default() -> Self {
        Self {
            feature: Capability::Stream,
            format: Format::CubeData,
            base_url: String::from("CHANGEME"),
            duration: 0,
            src: String::from("RID"),
        }
    }
}

#[ractor::async_trait]
impl AsyncStreamable for Cube {
    fn name(&self) -> String {
        String::from("AvionixCube")
    }

    async fn authenticate(&self) -> eyre::Result<String, AuthError> {
        trace!("fake token retrieval");
        Ok(String::from(""))
    }

    /// The main stream function, inspired by Opensky one.
    ///
    /// No cache is needed because it is plain TCP streaming.
    ///
    #[tracing::instrument(skip(self, out))]
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> eyre::Result<()> {
        trace!("avionixcube::stream");

        let filter = Filter::from(args);

        let stream_duration = match filter {
            Filter::Altitude { duration, .. } => Duration::from_secs(duration as u64),
            _ => Duration::new(0, 0),
        };

        trace!("Streaming data from {}…", self.base_url);

        info!(
            r##"
StreamURL: {}
Duration {}s
        "##,
            self.base_url,
            stream_duration.as_secs()
        );

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
        let tag = String::from("avionixcube::supervisor");
        let (sup, _h) = Actor::spawn(Some(tag), Supervisor, ()).await?;

        // Start the stats gathering actor.
        //
        trace!("starting stats actor.");
        let tag = String::from("avionix::stats");
        let (stat, _h) =
            Actor::spawn_linked(Some(tag), StatsActor, "avionixcube".into(), sup.get_cell())
                .await?;

        // Launch the worker actor
        //
        let url = format!("tcp://{}", self.base_url.clone());
        // Do not forget port is there is none specified.
        //
        let url = match Url::from_str(&url)?.port() {
            Some(_) => url,
            None => format!("{}:{}", url, DEF_PORT),
        };

        trace!("Starting worker actor.");
        let args = WorkerArgs {
            url,
            traffic: self.src.clone(),
            out,
            stat: stat.clone(),
        };
        let tag = String::from("avionixcube::worker");
        let (worker, _handle) =
            Actor::spawn_linked(Some(tag), LocalWorker, args, sup.get_cell()).await?;

        // Every TICK, we display stats.
        //
        stat.send_interval(TICK, || StatsMsg::Print);

        // Insert each actor in the PG_SOURCES group.
        //
        join(
            SOURCES_PG.into(),
            vec![sup.get_cell(), worker.get_cell(), stat.get_cell()],
        );

        info!("List of actors.");
        let list = pg::get_members(&SOURCES_PG.to_string());
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
        trace!("avionixcube::stream stopping.");

        // Stop everyone in the group.
        //
        pg::get_members(&SOURCES_PG.to_string())
            .iter()
            .for_each(|member| {
                member.stop(Some("Ending.".to_string()));
            });

        Ok(())
    }

    fn format(&self) -> Format {
        self.format
    }
}
