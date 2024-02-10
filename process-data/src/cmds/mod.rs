pub use acute::*;
pub use distances::*;
use duckdb::{Config, Connection};
pub use export::*;
pub use setup::*;

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

/// Connect to database.
///
pub fn connect_db(name: &str) -> eyre::Result<Connection> {
    info!("Connecting to {}", name);
    let dbh = Connection::open_with_flags(
        &name,
        Config::default()
            .allow_unsigned_extensions()?
            .enable_autoload_extension(true)?,
    )?;

    println!("Load extensions.");
    let _ = load_extensions(&dbh)?;
    Ok(dbh)
}
