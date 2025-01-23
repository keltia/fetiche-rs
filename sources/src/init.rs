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
pub fn init_sources_runtime() -> Result<Context> {
    let rt = tokio::runtime::Handle::current();
    let (stat, sup) = rt.block_on(async {
        // We have a generic supervisor actor.
        //
        trace!("starting supervisor actor.");
        let tag = String::from("senhive:supervisor");
        let (sup, _h) = Actor::spawn(Some(tag), Supervisor, ()).await.unwrap();

        // Start the stats gathering actor.
        //
        trace!("starting stats actor.");
        let tag = String::from("senhive::stats");
        let (stat, _h) =
            Actor::spawn_linked(Some(tag), StatsActor, "senhive".into(), sup.get_cell()).await.unwrap();
        (stat, sup)
    });
    Ok(Context {
        supervisor: sup,
        stats: stat,
    })
}
