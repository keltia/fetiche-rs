//! Job component of the Fetiche engine
//!
//! A `Job` consists of one or several tasks, all of which MUST be `Runnable`.
//!
use std::collections::VecDeque;

use anyhow::Result;
use log::{debug, error, trace};

use crate::Runnable;

/// The engine is processing jobs, made of runnable tasks
///
#[derive(Debug)]
pub struct Job {
    /// Name of the job
    pub name: String,
    /// FIFO list of tasks
    pub list: VecDeque<Box<dyn Runnable>>,
}

impl Job {
    /// New job
    ///
    /// NOTE: No //EOJ
    ///
    #[inline]
    pub fn new(name: &str) -> Self {
        trace!("Job::new");
        Job {
            name: name.to_owned(),
            list: VecDeque::new(),
        }
    }

    /// Add a task to the queue
    ///
    #[inline]
    pub fn add(&mut self, t: Box<dyn Runnable>) -> &mut Self {
        trace!("Job::add({t:?}");
        let _ = &self.list.push_back(t);
        self
    }

    /// Run all tasks and accumulate results into a single string
    ///
    pub fn run(&self) -> Result<String> {
        trace!("Job::run({})", self.name);

        // Gather result for all tasks into a single string using `Iterator::fold`
        //
        let res: String = self.list.iter().fold(String::new(), |res, t| {
            let r = t.run();
            match r {
                Ok(str) => res + str.as_str(),
                Err(e) => {
                    error!("task {:?}: {}", t, e.to_string());
                    res + ""
                }
            }
        });
        debug!("{:?}", res);
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Message, Nothing};

    use super::*;

    #[test]
    fn test_job_run() {
        let t1 = Box::new(Nothing {});
        let t2 = Box::new(Message::new("hello world"));

        let mut j = Job::new("test");
        j.add(t1);
        j.add(t2);

        let res = j.run();
        assert!(res.is_ok());
        assert_eq!("NOPhello world", res.unwrap())
    }
}
