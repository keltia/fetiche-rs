use std::fmt::Debug;

use duckdb::Connection;
use tracing::trace;

use crate::cmds::Stats;

/// This trait define an object that can be calculated
///
pub trait Calculate: Debug {
    fn run(&self, dbh: &Connection) -> eyre::Result<Stats>;
}

// -----

/// This is a batch, that is, a series on tasks that can be `Calculate`d using the corresponding
/// trait.  It also stores the handle to the database.
///
#[derive(Debug)]
pub struct Batch<'a, T>
    where T: Debug + Calculate,
{
    dbh: &'a Connection,
    inner: Vec<&'a T>,
}

impl<'a, T> Batch<'a, T>
    where T: Debug + Calculate,
{
    /// Create a new empty batch
    ///
    #[tracing::instrument]
    pub fn new(dbh: &'a Connection) -> Self {
        Self {
            dbh,
            inner: vec![],
        }
    }

    /// Add a single task t a batch
    ///
    /// Example:
    /// ```no_run
    /// let mut batch = Batch::new(dbh);
    ///
    /// let t1 = Task::new();
    /// let t2 = Task::new();
    /// batch.add(t1);
    /// batch.add(t2);
    /// ```
    ///
    #[tracing::instrument(skip(self))]
    pub fn add(&mut self, task: &'a T) -> &mut Self
    {
        self.inner.push(task);
        self
    }

    /// Create a batch from a vector of tasks
    ///
    /// Example:
    /// ```no_run
    /// let mut batch = Batch::new(dbh);
    ///
    /// let t1 = Task::new();
    /// let t2 = Task::new();
    /// batch.from_vec(vec![t1, t2]);
    /// ```
    ///
    #[tracing::instrument(skip(dbh))]
    pub fn from_vec(dbh: &'a Connection, v: &'a Vec<T>) -> Self
        where T: Debug + Calculate,
    {
        let mut b = Batch::new(dbh);
        v.iter().for_each(|elem| { b.add(elem); });
        b
    }

    /// Run all the tasks in sequence, gathering stats for each run in a vector.
    ///
    /// Example:
    /// ```no_run
    /// let mut batch = Batch::new(dbh);
    ///
    /// let t1 = Task::new();
    /// let t2 = Task::new();
    /// batch.from_vec(vec![t1, t2]);
    ///
    /// let stats: Vec<Stats> = batch.execute()?;
    /// ```
    ///
    #[tracing::instrument(skip(self))]
    pub fn execute(&mut self) -> eyre::Result<Vec<Stats>>
        where T: Debug + Calculate,
    {
        let dbh = self.dbh;

        let all: Vec<_> = self.inner.iter()
            .filter_map(|e| {
                let r = e.run(dbh);
                match r {
                    Ok(r) => Some(r),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        None
                    }
                }
            }).collect();

        trace!("all stats={:?}", all);
        Ok(all)
    }

    /// Returns the length of the current batch.
    ///
    #[tracing::instrument(skip(self))]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Return whether a batch is empty.
    ///
    #[tracing::instrument(skip(self))]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;

    use crate::cmds::PlanesStats;

    use super::*;

    #[derive(Debug)]
    struct Task {}

    impl Task {
        fn new() -> Self {
            Task {}
        }
    }

    impl Calculate for Task {
        fn run(&self, dbh: &Connection) -> eyre::Result<Stats> {
            let mut r = thread_rng();
            let val: u8 = r.gen();
            let upd: u8 = r.gen();
            let tm: u8 = r.gen();

            let hs = PlanesStats::default();
            let s = Stats::Planes(hs);
            eprintln!("calculate");
            Ok(s)
        }
    }

    #[test]
    fn test_batch_new() -> eyre::Result<()> {
        let dbh = Connection::open_in_memory()?;

        let b = Batch::<Task>::new(&dbh);
        assert!(b.inner.is_empty());
        Ok(())
    }

    #[test]
    fn test_batch_add() -> eyre::Result<()> {
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
    fn test_batch_from_vec() -> eyre::Result<()> {
        let dbh = Connection::open_in_memory()?;

        let t1 = Task::new();
        let t2 = Task::new();

        let tasks = vec![t1, t2];
        let b = Batch::from_vec(&dbh, &tasks);
        assert_eq!(2, b.len());
        Ok(())
    }

    #[test]
    fn test_batch_execute() -> eyre::Result<()> {
        let dbh = Connection::open_in_memory()?;

        let t1 = Task::new();
        let t2 = Task::new();

        let tasks = vec![t1, t2];
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
