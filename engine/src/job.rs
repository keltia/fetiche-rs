//! Job component of the Fetiche engine
//!
//! A `Job` consists of one or several tasks, all of which MUST be `Runnable`.
//! There is no real `stdin` for the first program in the pipe for now, first is
//! supposed to be collecting data (like `fetch` or `stream`) and send it along
//! the pipe for processing.
//!
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc::channel;

use anyhow::Result;
use log::trace;

use fetiche_sources::Sources;

use crate::Runnable;

/// The engine is processing jobs, made of runnable tasks
///
#[derive(Debug)]
pub struct Job {
    /// Source parameters
    pub srcs: Arc<Sources>,
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
    pub fn new(name: &str, srcs: Arc<Sources>) -> Self {
        trace!("Job::new");
        Job {
            srcs: Arc::clone(&srcs),
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
        trace!("Job::run({}) with {} tasks", self.name, self.list.len());

        // Setup the pipeline
        //
        let (key, stdout) = channel::<String>();

        // Gather results for all tasks into a single pipeline using `Iterator::fold()`
        //
        let output = self.list.iter_mut().fold(stdout, |acc, t| {
            let (rx, _) = t.run(acc);
            rx
        });

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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Task;

    use super::*;

    #[test]
    fn test_job_run() {
        let t1 = Box::new(Task::Nothing {});
        let t2 = Box::new(Task::Message::new("hello world"));

        let mut j: Job = Job::new("test");
        j.add(t1);
        j.add(t2);

        let mut data = vec![];

        let res = j.run(&mut data);

        let res = String::from_utf8(data);
        assert!(res.is_ok());
        assert_eq!("NOPhello world", res.unwrap())
    }
}
