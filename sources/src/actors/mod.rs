//! This module will have the various Actors this crate use.  Various system-dependant actors will
//! access these.
//!
//! Actors:
//!
//! `Stats`
//!
//! This actor accumulates statistics about packets/bytes received, etc.
//!
//! `Supervisor`
//!
//! This actor will be the father of all actors spawned by `sources`.
//!

pub use stats::*;
pub use supervisor::*;

mod stats;
mod supervisor;

use eyre::Result;
use ractor::{Actor, ActorRef};
use tracing::trace;

/// Name of the Actor "process group"
pub const PG_SOURCES: &str = "fetiche_sources";

#[derive(Clone, Debug)]
pub struct Context {
    pub supervisor: ActorRef<SuperMsg>,
    pub stats: ActorRef<StatsMsg>,
}

/// Initializes the runtime environment for source actors.
///
/// This function sets up the necessary actors for managing runtime behavior, such as a
/// generic supervisor actor and a stats gathering actor. The supervisor actor acts as the
/// main orchestrator, while the stats actor is responsible for collecting system statistics
/// related to the sources runtime.
///
/// # Returns
///
/// Returns a [`Context`] containing:
/// - A reference to the supervisor actor.
/// - A reference to the stats gathering actor.
///
/// # Errors
///
/// Returns an error if any actor fails to initialize or if there's an issue during actor spawning.
///
/// # Example
///
/// ```no_run
/// use eyre::Result;
/// use fetiche_sources::init_sources_runtime;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
/// let context = init_sources_runtime()?;
///     println!("Supervisor actor: {:?}", context.supervisor);
///     println!("Stats actor: {:?}", context.stats);
///     Ok(())
/// }
/// ```
///
pub async fn init_sources_runtime() -> Result<Context> {
    // We have a generic supervisor actor.
    //
    trace!("starting supervisor actor.");
    let tag = String::from("sources:supervisor");
    let (sup, _h) = Actor::spawn(Some(tag), Supervisor, ()).await.unwrap();

    // Start the stats gathering actor.
    //
    trace!("starting stats actor.");
    let tag = String::from("sources::stats");
    let (stat, _h) = Actor::spawn_linked(Some(tag), StatsActor, "sources".into(), sup.get_cell())
        .await
        .unwrap();
    Ok(Context {
        supervisor: sup,
        stats: stat,
    })
}
