//! This contains the common code for initialising the actors.
//!

use eyre::Result;
use ractor::{Actor, ActorRef};
use tracing::trace;

use crate::actors::{StatsActor, StatsMsg, Supervisor};

#[derive(Clone, Debug)]
pub struct Context {
    pub supervisor: ActorRef<()>,
    pub stats: ActorRef<StatsMsg>,
}

pub async fn init_sources_runtime() -> Result<Context> {
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

    Ok(Context {
        supervisor: sup,
        stats: stat,
    })
}
