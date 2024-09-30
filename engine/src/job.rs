//! Job component of the Fetiche engine
//!
//! A `Job` consists of one or several tasks, all of which MUST be `Runnable`.
//! There is no real `stdin` for the first program in the pipe for now, first is
//! supposed to be collecting data (like `fetch` or `stream`) and send it along
//! the pipe for processing.
//!
use std::collections::VecDeque;
use std::io::Write;
use std::sync::mpsc::channel;

use eyre::Result;
use tracing::{info, trace};
use tracing::{span, Level};

use crate::{EngineStatus, Runnable, IO};

/// The engine is processing jobs, made of runnable tasks
///
#[derive(Debug)]
pub struct Job {
    /// Job ID
    pub id: usize,
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
    #[tracing::instrument]
    #[inline]
    pub fn new(name: &str) -> Self {
        trace!("Job::new()");
        Self {
            id: 0,
            name: name.to_owned(),
            list: VecDeque::new(),
        }
    }

    /// Create job with a specific ID
    ///
    #[tracing::instrument]
    #[inline]
    pub fn new_with_id(name: &str, id: usize) -> Self {
        trace!("job({}) with id {}", name, id);
        Self {
            id,
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
                    return Err(EngineStatus::NoFirstProducer.into());
                }
            }
            None => return Err(EngineStatus::EmptyTaskList.into()),
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
                return Err(EngineStatus::NoLastConsumer.into());
            }
        }

        // Set the pipeline up
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
        trace!("pipe finished.");
        Ok(out.flush()?)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Copy, Engine, Message, Nothing};

    use super::*;

    #[test]
    fn test_job_run_nothing() {
        let mut e = Engine::new();
        let t1 = Box::new(Nothing::new());
        let t2 = Box::new(Copy::new());

        let mut j: Job = e.create_job("test");
        j.add(t1);
        j.add(t2);

        let mut data = vec![];

        let res = j.run(&mut data);
        assert!(res.is_ok());

        let res = String::from_utf8(data);
        assert!(res.is_ok());
        assert_eq!("start|NOP", res.unwrap())
    }

    #[test]
    fn test_job_run_message() {
        let mut e = Engine::new();
        let t1 = Box::new(Message::new("hello world"));
        let t2 = Box::new(Copy::new());

        let mut j: Job = e.create_job("test");
        j.add(t1);
        j.add(t2);

        let mut data = vec![];

        let res = j.run(&mut data);
        assert!(res.is_ok());

        let res = String::from_utf8(data);
        assert!(res.is_ok());
        assert_eq!("hello world", res.unwrap())
    }
}
