use std::fmt::Debug;
use std::sync::Arc;
use duckdb::Connection;
use crate::cmds::Stats;

/// This trait define an object that can be calculated
///
pub trait Calculate: Debug {
    fn run(&self, dbh: &Connection) -> eyre::Result<Stats>;
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
    pub fn execute(&mut self) -> eyre::Result<Vec<Stats>>
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
        fn new() -> Self {
            Task {}
        }
    }

    impl Calculate for Task {
        fn run(&self, dbh: &Connection) -> eyre::Result<Stats> {
            let mut r = thread_rng();
            let val: u8 = r.gen();

            let hs = HomeStats { distances: val as usize };
            let s = Stats::Home(hs);
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
