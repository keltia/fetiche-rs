//! Job component of the Fetiche engine
//!
//! A `Job` consists of one or several tasks, all of which MUST be `Runnable`.
//! There is no real `stdin` for the first program in the pipe for now, first is
//! supposed to be collecting data (like `fetch` or `stream`) and send it along
//! the pipe for processing.
//!
//! A `Work` is a submitted job, created when submitting.
//! A `WaitGroup` is the struct you get after submission to be notified when the job has finished.
//!
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};

use derive_builder::Builder;
use ractor::ActorRef;

use crate::actors::StatsMsg;
use crate::{Consumer, Middle, Producer};

/// This gets sent back to the caller when submitting the job.
///
#[derive(Debug)]
pub struct WaitGroup {
    /// ID of the job
    pub id: usize,
    /// Synchronisation point
    pub rx: Receiver<()>,
}

impl WaitGroup {
    #[tracing::instrument(skip(rx))]
    pub fn new(id: usize, rx: Receiver<()>) -> Self {
        Self { id, rx }
    }
}

/// This is the other part which is queued for execution.
///
/// Represents a unit of work that can be queued for execution.
///
/// This struct encapsulates a job and its associated synchronization channel.
/// It is used internally by the engine to manage job execution and state updates.
///
/// # Fields
///
/// * `job` - The job to be executed
/// * `tx` - A sender channel used for synchronization when the job completes
///
#[derive(Clone, Debug)]
pub struct Work {
    pub job: Job,
    pub tx: Sender<()>,
}

impl Work {
    /// Creates a new Work instance.
    ///
    /// # Arguments
    ///
    /// * `job` - The job to be executed
    /// * `tx` - A sender channel for job completion notification
    ///
    #[tracing::instrument(skip(tx))]
    pub fn new(job: Job, tx: Sender<()>) -> Self {
        Self { job, tx }
    }

    /// Returns the current state of the contained job.
    ///
    #[tracing::instrument(skip(self))]
    pub fn state(&self) -> JobState {
        self.job.state()
    }

    /// Returns the ID of the contained job.
    ///
    #[tracing::instrument(skip(self))]
    pub fn id(&self) -> usize {
        self.job.id
    }
}

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
    /// FIFO list of middle tasks
    #[builder(default = "VecDeque::new()")]
    pub middle: VecDeque<Middle>,
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
    /// Empty, just allocated
    #[default]
    Created,
    /// Has all its tasks
    Ready,
    /// Executing
    Running,
    /// Finished
    Completed,
    /// Weird
    Zombie,
}

impl Job {
    /// Adds a middleware middle task to the job's pipeline.
    ///
    /// This method appends a new middle task to the end of the job's middle queue,
    /// maintaining the FIFO (First In, First Out) order of execution. Each middle
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
        let _ = &self.middle.push_back(t);
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
        assert!(job.middle.is_empty());
    }

    #[test]
    fn test_new_job_with_id_empty_name() {
        let job = JobBuilder::default()
            .name("empty".into())
            .id(1)
            .build()
            .unwrap();

        assert_eq!(job.name, "empty");
        assert!(job.middle.is_empty());
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
        assert!(job.middle.is_empty());

        job.add(Middle::Invalid);
        assert_eq!(job.middle.len(), 1);

        job.add(Middle::Invalid);
        assert_eq!(job.middle.len(), 2);
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
