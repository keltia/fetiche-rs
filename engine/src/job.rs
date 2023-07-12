//! Job component of the Fetiche engine
//!
//! A `Job` consists of one or several tasks, all of which MUST be `Runnable`.
//! There is no real `stdin` for the first program in the pipe for now, first is
//! supposed to be collecting data (like `fetch` or `stream`) and send it along
//! the pipe for processing.
//!
use std::collections::{BTreeMap, VecDeque};
use std::io::Write;
use std::sync::mpsc::channel;

use anyhow::{anyhow, Result};
use tracing::{info, trace};
use tracing::{span, Level};
use uuid::Uuid;

use crate::{Runnable, IO};

/// The engine is processing jobs, made of runnable tasks
///
#[derive(Debug)]
pub struct Job {
    /// Job ID
    pub id: String,
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
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        let uuid = Uuid::new_v4().to_string();
        trace!("Job::new({})", uuid);
        Job {
            id: uuid,
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

    /// Run all tasks and accumulate results into a single stream
    ///
    /// For each task, `run()` create a channel, launch a thread for the task and pass the receiver
    /// to the next thread.
    ///
    /// The returned value is the last "output" channel which is the result of the pipeline run.
    ///
    /// For now we ignore the handle for all threads, should we store them and `join()` later?  They
    /// are launched in parallel but each one depends on the reading of the "in" pipe.
    ///
    /// By using only channels between all threads, we should avoid any issues with passing something
    /// more complicated like we did with `out`.
    ///
    pub fn run(&mut self, out: &mut dyn Write) -> Result<()> {
        let span = span!(Level::TRACE, "job::run");
        let _ = span.enter();

        info!(
            "Job({})::run({}) with {} tasks",
            self.id,
            self.name,
            self.list.len()
        );

        // Basic checks on the pipeline
        //
        let first = &self.list.front();
        let last = &self.list.back();

        match first {
            Some(first) => {
                if first.cap() != IO::Producer {
                    return Err(anyhow!("First task must be a producer"));
                }
            }
            None => return Err(anyhow!("empty task list")),
        }

        // At this point, `self.list` is not empty so in the worst case, `first == last`.
        //
        let last = last.unwrap();

        // If there is only one task, it should be fine.
        //
        if self.list.len() != 1 {
            // Then we check the last one
            //
            if last.cap() != IO::Consumer && last.cap() != IO::Filter {
                return Err(anyhow!("last must be consumer or filter"));
            }
        }

        // Setup the pipeline
        //
        let (key, stdout) = channel::<String>();

        trace!("create pipeline");

        // Gather results for all tasks into a single pipeline using `Iterator::fold()`
        //
        let output = self.list.iter_mut().fold(stdout, |acc, t| {
            let (rx, _) = t.run(acc);
            rx
        });

        trace!("starting pipe");

        // Start the pipeline
        //
        key.send("start".to_string())?;

        // Close the pipeline which will stop all threads in sequence
        //
        drop(key);

        // Wait for final output to be received and send it out
        //
        for msg in output {
            write!(out, "{}", msg)?;
        }
        Ok(out.flush()?)
    }
}

/// Job queue
///
#[derive(Debug)]
pub struct Jobs(BTreeMap<String, Job>);

#[cfg(test)]
mod tests {
    use crate::{Engine, Message, Nothing};

    use super::*;

    #[test]
    fn test_job_run() {
        env_logger::init();

        let e = Engine::new();
        let t1 = Box::new(Nothing::new());
        let t2 = Box::new(Message::new("hello world"));

        let mut j: Job = e.create_job("test");
        j.add(t1);
        j.add(t2);

        let mut data = vec![];

        let res = j.run(&mut data);
        assert!(res.is_ok());

        let res = String::from_utf8(data);
        assert!(res.is_ok());
        assert_eq!("start|NOP|hello world", res.unwrap())
    }
}
