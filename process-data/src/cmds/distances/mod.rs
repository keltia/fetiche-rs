use std::fmt::Debug;
use std::sync::Arc;

use clap::Parser;
use duckdb::Connection;
use eyre::Result;

pub use home::*;
pub use planes::*;

use crate::cmds::Stats;

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
pub struct Batch<'a, T>
    where T: Debug + Calculate,
{
    dbh: Arc<&'a Connection>,
    inner: Vec<&'a T>,
}

impl<'a, T> Batch<'a, T>
    where T: Debug + Calculate,
{
    #[tracing::instrument]
    pub fn new(dbh: &'a Connection) -> Self {
        Self {
            dbh: Arc::new(dbh),
            inner: vec![],
        }
    }

    #[tracing::instrument]
    pub fn add(&mut self, task: &'a T) -> &mut Self
    {
        self.inner.push(task);
        self
    }

    #[tracing::instrument]
    pub fn from_vec(dbh: &'a Connection, v: &'a Vec<T>) -> Self
        where T: Debug + Calculate,
    {
        let mut b = Batch::new(&dbh);
        v.into_iter().for_each(|elem| { b.add(elem); });
        b
    }

    #[tracing::instrument]
    pub fn execute(&mut self) -> Result<Vec<Stats>>
        where T: Debug + Calculate,
    {
        let dbh = self.dbh.clone();

        let all: Vec<_> = self.inner.iter()
            .filter_map(|e| {
                let r = e.run(&dbh);
                match r {
                    Ok(r) => Some(r),
                    Err(e) => {
                        eprintln!("Error: {}", e.to_string());
                        None
                    }
                }
            }).collect();

        dbg!(&all);
        Ok(all)
    }

    #[tracing::instrument]
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;

    use crate::cmds::HomeStats;

    use super::*;

    #[derive(Debug)]
    struct Task {}

    impl Task {
        fn new() -> Box<dyn Calculate> {
            Box::new(Task {})
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

        b.add(&t1);
        b.add(&t2);
        assert_eq!(2, b.len());
        Ok(())
    }

    #[test]
    fn test_batch_from_vec() -> Result<()> {
        let dbh = Connection::open_in_memory()?;

        let t1 = Task::new();
        let t2 = Task::new();

        let tasks = [t1, t2];
        let mut b = Batch::from_vec(&dbh, &tasks);
        assert_eq!(2, b.len());
        Ok(())
    }

    #[test]
    fn test_batch_execute() -> Result<()> {
        let dbh = Connection::open_in_memory()?;

        let t1 = Task::new();
        let t2 = Task::new();

        let tasks = [t1, t2];
        let mut b = Batch::from_vec(&dbh, &tasks);
        dbg!(&b);

        let v = b.execute()?;
        dbg!(&v);
        assert_eq!(2, v.len());

        let summ = Stats::summarise(v);
        dbg!(&summ);

        Ok(())
    }
}
