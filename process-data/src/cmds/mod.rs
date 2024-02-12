pub use acute::*;
pub use distances::*;
pub use export::*;
pub use setup::*;

use duckdb::{Config, Connection};
use thiserror::Error;
use tracing::info;

mod acute;
mod distances;
mod export;
mod setup;

/// One degree in *kilometers*
const ONE_DEG: f64 = 40_000. / 360.;

#[derive(Debug, Error)]
pub enum Status {
    #[error("No planes were found around site {0} at this date")]
    NoPlanesFound(String),
    #[error("No drones in the {0} area")]
    NoDronesFound(String),
    #[error("No encounters found in the {0} area")]
    NoEncounters(String),
    #[error("Invalid site name {0}")]
    ErrUnknownSite(String),
}

/// Connect to database and load the extensions.
///
#[tracing::instrument]
pub fn init_runtime(name: &str) -> eyre::Result<Connection> {
    info!("Connecting to {}", name);
    let dbh = Connection::open_with_flags(
        name,
        Config::default()
            .allow_unsigned_extensions()?
            .enable_autoload_extension(true)?,
    )?;

    println!("Load extensions.");
    load_extensions(&dbh)?;
    Ok(dbh)
}

/// We need these extensions all the time.
///
#[tracing::instrument(skip(dbh))]
pub fn load_extensions(dbh: &Connection) -> eyre::Result<()> {
    // Load our extensions
    //
    dbh.execute("LOAD spatial", [])?;
    Ok(())
}
