use std::fmt::Debug;
use std::sync::Arc;

use clap::Parser;
use duckdb::Connection;
use eyre::Result;

pub use home::*;
pub use planes::*;

use crate::cmds::{PlanesStats, Stats};

mod home;
mod planes;

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
    fn run(&self, dbh: &Connection) -> Result<Stats>;
}

// -----

#[derive(Debug)]
pub struct Batch<'a> {
    dbh: Arc<&'a Connection>,
    inner: Vec<Box<dyn Calculate>>,
}

impl<'a> Batch<'a> {
    pub fn new(dbh: &'a Connection) -> Self {
        Self {
            dbh: Arc::new(dbh),
            inner: vec![],
        }
    }

    #[tracing::instrument]
    pub fn add(&mut self, task: Box<dyn Calculate>) -> &mut Self {
        self.inner.push(task);
        self
    }

    #[tracing::instrument]
    pub fn execute(&mut self) -> Result<Vec<Stats>> {
        let dbh = self.dbh.clone();

        let list = self.inner.iter();

        let (all, errors): (Vec<_>, Vec<_>) = list
            .map(|e| {
                e.run(&dbh)
            })
            .partition(|x| { x.is_ok() });

        eprintln!("errors={:?}", errors);

        let all = all.into_iter().map(|e| e.unwrap()).collect();
        Ok(all)
    }

    #[tracing::instrument]
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<const N: usize> From<[Box<dyn Calculate>; N]> for Batch {
    fn from(value: [Box<dyn Calculate>; N]) -> Self {
        let a = Batch::new(&dbh)
        value.iter().for_each()
    }
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;
    use super::*;

    use crate::cmds::HomeStats;

    #[derive(Debug)]
    struct Task {}

    impl Task {
        fn new() -> Self {
            Task {}
        }
    }

    impl Calculate for Task {
        fn run(&self, dbh: &Connection) -> Result<Stats> {
            let mut r = thread_rng();
            let val: u8 = r.gen();

            let hs = HomeStats { distances: val as usize };
            let s = Stats::Home(hs);
            eprintln!("calculate");
            Ok(s)
        }
    }

    #[test]
    fn test_batch_new() -> Result<()> {
        let dbh = Connection::open_in_memory()?;

        let b = Batch::new(&dbh);
        assert!(b.inner.is_empty());
        Ok(())
    }

    #[test]
    fn test_batch_add() -> Result<()> {
        let dbh = Connection::open_in_memory()?;

        let mut b = Batch::new(&dbh);
        let t1 = Task::new();
        let t2 = Task::new();

        b.add(Box::new(t1));
        b.add(Box::new(t2));
        assert_eq!(2, b.len());
        Ok(())
    }

    #[test]
    fn test_batch_add() -> Result<()> {
        let dbh = Connection::open_in_memory()?;

        let mut b = Batch::new(&dbh);
        let t1 = Task::new();
        let t2 = Task::new();

        b.add(Box::new(t1));
        b.add(Box::new(t2));
        assert_eq!(2, b.len());

        let s = b.execute()?;
        dbg!(&s);
        assert_eq!(2, s.len());

        let summ = s.iter().fold(Stats::Home::de, ||)
        Ok(())
    }
}
