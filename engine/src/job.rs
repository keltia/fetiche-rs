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

use crate::{EngineStatus, Runnable, Task, IO};
use eyre::Result;
use fetiche_sources::Site;
use tracing::{info, span, trace, Level};

/// A `Job` represents a pipeline of tasks to be executed sequentially.
///
/// The `Job` is part of the Fetiche engine and is used to define
/// and manage a sequence of operations (tasks) that are executed in order,
/// where each task processes and passes data to the next one in a producer-consumer chain.
///
/// # Overview
/// A `Job` consists of:
/// - A `id` allocated at creation-time.
/// - A `name` for recognizing the job's purpose or type.
/// - A `state` to see the lifetime of the job
/// - A `list` of tasks to be executed in order, backed by a `VecDeque` for FIFO (First In, First Out) behavior.
///
/// Tasks in the job must implement the `Runnable` trait and adhere to its constraints.
/// The pipeline allows chaining tasks that transform data from an initial producer to a final consumer.
///
/// # Attributes
/// - `id`: Job ID
/// - `name`: A descriptive name for the `Job`.
/// - `state`: The status of the given job.
/// - `list`: A list (queue) of tasks to be executed in the pipeline.
///
/// # Usage
/// Jobs are created using the `Job::new` or `Job::new_with_id` methods.
/// Tasks can be added to the job using `Job::add` and run sequentially using `Job::run`.
///
/// # Example
/// ```rust
/// use std::io::Cursor;
/// use fetiche_engine::{Job, Nothing, Task};
///
/// // Create a new Job
/// let mut job = Job::new("Example Pipeline");
///
/// // Add a simple task
/// let nop = Nothing::new();
/// let task = Task::from(nop);
/// job.add(task);
///
/// // Prepare output writer
/// let mut output = Cursor::new(Vec::new());
///
/// // Execute job
/// let result = job.run(&mut output);
/// assert!(result.is_ok());
/// ```
///
#[derive(Clone, Debug)]
pub struct Job {
    /// Job ID
    pub id: usize,
    /// Name of the job
    pub name: String,
    /// Job State
    pub state: JobState,
    /// FIFO list of tasks
    pub list: VecDeque<Task>,
}

/// Represents the different states that a Job can be in during its lifecycle.
///
/// # States
/// - `Created`: Initial state when the job is first allocated but has no tasks.
/// - `Ready`: Job has been populated with all required tasks and is ready to be queued.
/// - `Queued`: Job has been placed in the execution queue and is waiting to be run.
/// - `Running`: Job is currently being executed, with its tasks being processed.
/// - `Completed`: Job has finished executing all its tasks successfully.
/// - `Zombie`: Job is in an invalid or unexpected state, typically after an error.
///
#[derive(Clone, Debug, PartialEq)]
pub enum JobState {
    /// Empty, just allocated
    Created,
    /// Has all its tasks
    Ready,
    /// In the queue for next run
    Queued,
    /// Executing
    Running,
    /// Finished
    Completed,
    /// Weird
    Zombie,
}

impl Job {
    /// New job
    ///
    /// NOTE: No //EOJ
    ///
    #[tracing::instrument]
    #[inline]
    pub fn new(name: &str, id: usize) -> Self {
        trace!("Job::new()");
        Self {
            id,
            name: name.to_owned(),
            state: JobState::Created,
            list: VecDeque::new(),
        }
    }

    /// Add a task to the queue
    ///
    #[tracing::instrument(skip(self))]
    #[inline]
    pub fn add(&mut self, t: Task) -> &mut Self {
        let _ = &self.list.push_back(t);
        self
    }

    /// Executes the tasks in the order they are stored in the pipeline and ensures that
    /// the output of one task flows properly to the following task, creating a producer-consumer chain.
    ///
    /// # Overview
    /// This function performs the following:
    /// 1. Validates the job's pipeline, ensuring it has a valid start and end.
    /// 2. Sequentially executes all tasks using their `run()` method.
    /// 3. Writes the final task's output into the provided `out` writer.
    ///
    /// # Parameters
    /// - `&mut self`: A mutable reference to the job object, enabling changes to its internal state as required.
    /// - `out: &mut dyn Write`: A mutable reference to a writer object where the final output will be stored.
    ///
    /// # Returns
    /// - `Ok(())`: Indicates success in running the pipeline to completion and writing the result to `out`.
    /// - `Err(anyhow::Error)`: Indicates errors during pipeline validation, task execution, or IO operations.
    ///
    /// # Errors
    /// This method can fail due to:
    /// 1. Pipeline validation errors:
    ///    - No tasks found in the pipeline.
    ///    - Missing a starting `IO::Producer` task or ending `IO::Filter` or `IO::Consumer` task.
    /// 2. Issues with inter-task communication or passing data between tasks.
    /// 3. Failures while writing the final output to the provided `out` writer.
    ///
    /// # Notes
    /// - Tasks in the pipeline must implement the `Runnable` trait and adhere to its constraints.
    /// - If the execution halts due to errors, the output writer may remain unchanged.
    /// - Useful for executing a sequence of dependent operations in a structured manner.
    ///   `out` writer.
    ///
    /// # Behavior and Execution Process
    /// 1. Logs the start of the `Job::run()` process for tracing purposes.
    /// 2. Validates the pipeline based on these rules:
    ///    - The first task must be a `Producer`.
    ///    - The pipeline must have at least one task.
    ///    - If there are multiple tasks, the last one must be a `Consumer` or a `Filter`.
    /// 3. Creates a communication channel (`channel`) through which task messages are passed.
    /// 4. Sequentially executes tasks using a `fold()` to chain their outputs.
    /// 5. Sends a "start" signal to kick off the pipeline, and then gracefully closes the input channel.
    /// 6. Collects the final output messages from the pipeline and writes them to the provided `out` writer.
    ///
    /// # Example Usage
    /// ```rust
    /// use std::io::Cursor;
    /// use fetiche_engine::{Job, Nothing, Task};
    ///
    /// // Create a sample job with tasks (details of tasks omitted)
    /// let mut job = Job::new("Example Job");
    /// let nop = Nothing::new();
    /// let task = Task::from(nop);
    /// job.add(task);
    ///
    /// // Prepare an output writer
    /// let mut output = Cursor::new(Vec::new());
    ///
    /// // Execute the job
    /// let result = job.run(&mut output);
    ///
    /// // Check results
    /// assert!(result.is_ok());
    /// let output_str = String::from_utf8(output.into_inner()).unwrap();
    /// println!("Job Output: {}", output_str);
    /// ```
    ///
    /// # Logging
    /// - The function uses `tracing::span` for detailed, structured tracing of execution flow.
    /// - High-level information is logged using `info!` (e.g., job ID, name, and task count).
    /// - Detailed information is logged using `trace!` (e.g., pipeline creation, pipeline completion).
    ///
    pub fn run(&mut self, out: &mut dyn Write) -> Result<()> {
        let span = span!(Level::TRACE, "job::run");
        let _ = span.enter();

        info!(
            "Job({})::run({}) with {} tasks",
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
    use crate::{Copy, Engine, Message};

    use super::*;

    #[test]
    fn test_new() {
        let job = Job::new("Test Job");

        assert_eq!(job.name, "Test Job");
        assert!(job.list.is_empty());
    }

    #[test]
    fn test_new_with_id_empty_name() {
        let job = Job::new("");

        assert_eq!(job.name, "");
        assert!(job.list.is_empty());
    }

    #[tokio::test]
    async fn test_job_run_message() {
        let mut e = Engine::new().await;
        let t1 = Task::from(Message::new("hello world"));
        let t2 = Task::from(Copy::new());

        let mut j = e.create_job("test").await.unwrap();
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
