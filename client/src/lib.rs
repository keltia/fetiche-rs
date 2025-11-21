//! Client library for fetiche-rs
//!
use eyre::Result;
use ractor::{call, Actor, ActorRef};
use tracing::{info, trace};

mod job;
mod local;
mod single;
mod sites;
mod supervisor;
mod workspace;

pub use job::*;
pub use local::*;
pub use single::*;
pub use sites::*;
pub use supervisor::*;
pub use workspace::*;

// Re-export engine stuff.
pub use fetiche_engine::{Filter, Freq, JobState};

/// Our main process group
pub const FETICHE_PG: &str = "fetiche.pg";

/// Client signature
///
#[tracing::instrument]
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// Client represents the main interface for interacting with the fetiche-rs system.
/// It manages the supervisor and sources actors that handle core functionality.
///
#[derive(Debug)]
pub struct Client {
    /// Reference to the supervisor actor handling overall system coordination
    sup: ActorRef<SuperMsg>,
    /// Reference to the sources actor managing data sources
    sites: ActorRef<SitesMsg>,
}

impl Client {
    /// Initializes a new Client instance by spawning and configuring required actors.
    ///
    /// # Returns
    /// * `Result<Self>` - A new Client instance if initialization succeeds, or an error if it fails
    ///
    /// # Errors
    /// Returns an error if actor spawning fails or if sources cannot be loaded
    ///
    #[tracing::instrument]
    pub async fn init() -> Result<Self> {
        trace!("starting client init");
        // We have a generic supervisor actor.
        //
        trace!("starting supervisor actor aka init.");
        let tag = String::from("init");
        let (sup, _h) = Actor::spawn(Some(tag), Supervisor, FETICHE_PG.into()).await?;

        // Start sites service
        //
        trace!("load sites");
        let (sites, _h) = Actor::spawn_linked(
            Some("fetiche::sites".into()),
            SitesActor,
            FETICHE_PG.into(),
            sup.get_cell(),
        )
        .await?;

        let count = call!(sites, SitesMsg::Count)?;
        info!("{} sources loaded", count);

        Ok(Self { sites, sup })
    }
}
