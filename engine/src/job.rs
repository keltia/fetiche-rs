//! Job component of the Fetiche engine
//!
//! A `Job` consists of one or several tasks, all of which MUST be `Runnable`.
//! There is no real `stdin` for the first program in the pipe for now, first is
//! supposed to be collecting data (like `fetch` or `stream`) and send it along
//! the pipe for processing.
//!
use std::collections::VecDeque;
use std::io::Write;
use std::vec;

use derive_builder::Builder;
use eyre::Result;
use ractor::ActorRef;
use tracing::{info, span, trace, Level};

use crate::actors::StatsMsg;
use crate::{Consumer, Middle, Producer, Runnable, Task};

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
#[derive(Builder, Clone, Debug)]
pub struct Job {
    /// Job ID
    pub id: usize,
    /// Name of the job
    #[builder(default = "String::from(\"Default Name\")")]
    pub name: String,
    /// Job State
    #[builder(default = "JobState::Created")]
    pub state: JobState,
    /// Producer.
    #[builder(default = "Producer::Invalid")]
    pub producer: Producer,
    /// FIFO list of filter tasks
    #[builder(default = "VecDeque::new()")]
    pub filters: VecDeque<Middle>,
    /// The end of the pipeline.
    #[builder(default = "Consumer::Invalid")]
    pub consumer: Consumer,
    /// actor for statistics
    #[builder(default)]
    pub stats: Option<ActorRef<StatsMsg>>,
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
#[derive(Clone, Debug, Default, PartialEq)]
pub enum JobState {
    #[default]
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
    /// Adds a middleware filter task to the job's pipeline.
    ///
    /// This method appends a new filter task to the end of the job's filter queue,
    /// maintaining the FIFO (First In, First Out) order of execution. Each filter
    /// added will be executed in the order they were added during the job's run.
    ///
    /// # Parameters
    /// - `t`: A `Middle` task to be added to the pipeline. This represents a data
    ///   transformation or processing step that will be executed as part of the job.
    ///
    /// # Returns
    /// - `&mut Self`: Returns a mutable reference to the Job instance, enabling method chaining.
    ///
    #[tracing::instrument(skip(self))]
    #[inline]
    pub fn add(&mut self, t: Middle) -> &mut Self {
        let _ = &self.filters.push_back(t);
        self
    }

    /// Registers a statistics actor with this job for collecting metrics during execution.
    ///
    /// This method sets up the connection between the job and a statistics collection actor
    /// that will receive updates about the job's execution progress and performance metrics.
    ///
    /// # Parameters
    /// - `t`: An `ActorRef<StatsMsg>` reference to the statistics actor that will collect metrics
    ///
    /// # Returns
    /// - `&mut Self`: Returns a mutable reference to the Job instance, enabling method chaining
    ///
    #[tracing::instrument(skip(self, t))]
    #[inline]
    pub fn register(&mut self, t: ActorRef<StatsMsg>) -> &mut Self {
        self.stats = Some(t);
        self
    }

    /// Updates the current state of the job.
    ///
    /// This method allows changing the job's state during its lifecycle,
    /// tracking its progression through different stages of execution.
    ///
    /// # Parameters
    /// - `s`: The new `JobState` to set for this job
    ///
    /// # Returns
    /// - `&mut Self`: Returns a mutable reference to the Job instance, enabling method chaining
    ///
    #[tracing::instrument(skip(self))]
    #[inline]
    pub fn set(&mut self, s: JobState) -> &mut Self {
        self.state = s;
        self
    }

    #[tracing::instrument(skip(self))]
    #[inline]
    /// Returns the current state of the job.
    ///
    /// This method provides access to the job's current state, allowing external code
    /// to check the job's progress through its lifecycle stages.
    ///
    /// # Returns
    /// - `JobState`: A clone of the job's current state enum value
    ///
    pub fn state(&self) -> JobState {
        self.state.clone()
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
            self.id,
            self.name,
            self.filters.len() + 2
        );

        // insert tasks into the pipeline
        //
        let first = vec![Task::from(self.producer.clone())];
        let filters = self.filters.iter().map(|&t| Task::from(t)).collect::<Vec<Task>>();
        let last = vec![Task::from(self.consumer.clone())];

        // Create our list of linked tasks
        //
        let mut task_list = [first, filters, last].into_iter().flatten().collect::<Vec<Task>>();

        // Set the pipeline up
        //
        let (key, stdout) = channel::<String>();

        trace!("create pipeline");

        // Gather results for all tasks into a single pipeline using `Iterator::fold()`
        //
        let output = task_list.iter_mut().fold(stdout, |acc, t| {
            let (rx, _h) = t.run(acc);
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
    use super::*;

    #[test]
    fn test_job_new() {
        let job = JobBuilder::default()
            .name("Test Job".into())
            .id(1)
            .build()
            .unwrap();

        assert_eq!(job.name, "Test Job");
        assert_eq!(job.id, 1);
        assert!(job.filters.is_empty());
    }

    #[test]
    fn test_new_job_with_id_empty_name() {
        let job = JobBuilder::default()
            .name("empty".into())
            .id(1)
            .build()
            .unwrap();

        assert_eq!(job.name, "empty");
        assert!(job.filters.is_empty());
    }

    #[test]
    fn test_job_state_transitions() {
        let mut job = JobBuilder::default().id(1).build().unwrap();
        assert_eq!(job.state(), JobState::Created);

        job.set(JobState::Ready);
        assert_eq!(job.state(), JobState::Ready);

        job.set(JobState::Running);
        assert_eq!(job.state(), JobState::Running);

        job.set(JobState::Completed);
        assert_eq!(job.state(), JobState::Completed);
    }

    #[test]
    fn test_job_add_filters() {
        let mut job = JobBuilder::default().id(1).build().unwrap();
        assert!(job.filters.is_empty());

        job.add(Middle::Invalid);
        assert_eq!(job.filters.len(), 1);

        job.add(Middle::Invalid);
        assert_eq!(job.filters.len(), 2);
    }

    #[test]
    fn test_default_job() {
        let job = JobBuilder::default().id(1).build().unwrap();
        assert_eq!(job.name, "Default Name");
        assert_eq!(job.state, JobState::Created);
        assert_eq!(job.producer, Producer::Invalid);
        assert_eq!(job.consumer, Consumer::Invalid);
        assert!(job.stats.is_none());
    }
}
