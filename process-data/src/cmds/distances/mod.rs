use std::fmt::Debug;
use std::sync::Arc;

use clap::Parser;
use duckdb::Connection;
use eyre::Result;
use tracing::error;

pub use to_home::*;
pub use to_planes::*;

use crate::cmds::{PlanesStats, Stats};

mod to_home;
mod to_planes;

#[derive(Debug, Parser)]
pub(crate) struct DistOpts {
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// `distances` sub-commands
    #[clap(subcommand)]
    pub subcmd: DistSubcommand,
}

#[derive(Clone, Debug, Parser)]
pub(crate) enum DistSubcommand {
    /// 2D/3D drone to operator distance.
    Home,
    /// drone to planes distance
    Planes(PlanesOpts),
}

// -----

/// This trait define an object that can be calculated
///
pub trait Calculate: Debug {
    fn execute(&self, dbh: &Connection) -> Result<Stats>;
}

// -----

#[derive(Clone, Debug)]
pub struct Batch<'a> {
    dbh: Arc<&'a Connection>,
    inner: Vec<Box<dyn Calculate>>,
}

impl<'a> Batch<'a> {
    pub fn new(dbh: &Connection) -> Self {
        Self { dbh: Arc::new(dbh.clone()), inner: vec![] }
    }

    #[tracing::instrument]
    pub fn add(&mut self, task: Box<dyn Calculate>) -> &mut Self {
        self.inner.push(task);
        self
    }

    #[tracing::instrument]
    pub fn execute(&mut self) -> Result<Vec<Stats>> {
        let dbh = self.dbh.clone();

        let all: Vec<Stats> = self.inner.iter().map(|&e| {
            match e.calculate(dbh) {
                Ok(stats) => stats,
                Err(e) => error!("Error: {}", e),
            }
        }).collect::<Vec<_>>();
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_new() -> Result<()> {
        let dbh = Connection::open_in_memory()?;

        let mut b = Batch::new(&dbh);
        assert!(b.inner.is_empty());

        let s: Vec<Stats> = b.execute()?;
        dbg!(&s);

        Ok(())
    }
}
