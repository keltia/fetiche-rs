//! Engine command implementation module
//!
//! This module implements the core command functionality for the engine, including:
//! - Job creation and management
//! - Job queue operations
//! - Job state transitions
//! - Job submission and execution
//!
//! The commands are implemented as methods on the `Engine` struct and provide the primary
//! interface for interacting with the engine's job processing capabilities.
//!
//! Each command ensures proper state management and provides appropriate error handling
//! when operations cannot be completed successfully. All operations are instrumented
//! with tracing for debugging and monitoring purposes.
//!

use std::collections::BTreeMap;

use ractor::factory::{FactoryMessage, JobOptions};
use ractor::{call, cast, factory};
use serde::Deserialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::{error, info, trace};

use crate::actors::{ResultsMsg, SchedulerMsg, StateMsg};
use crate::{ENGINE_PG, Engine, EngineStatus, IO, Job, JobBuilder, JobState, Stats, WaitGroup};

impl Engine {
    ///
    ///
    /// Create a new job
    ///
    /// This method creates a new job within the engine by utilizing the internal job queue mechanism.
    /// The created job is assigned a unique ID, initialized, and synchronized with the engine state.
    ///
    /// # Arguments
    ///
    /// - `s`: A string slice representing the job's description or identifier.
    ///
    /// # Returns
    ///
    /// - On success, returns `Ok(Job)` with the created job.
    /// - On failure, returns an `Err` containing details about the error.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use fetiche_engine::{Engine, JobState};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut engine = Engine::new().await?;
    ///
    ///     let job = engine.create_job("example_job")?;
    ///     println!("Job created with ID: {}", job.id);
    ///     assert_eq!(job.state, JobState::Created);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error in the following cases:
    ///
    /// - If the job cannot be added to the queue.
    /// - If the state service fails to update for the new job.
    /// - If synchronization fails with the engine state after job creation.
    ///
    /// # Tracing
    ///
    /// Tracing logs provide insights during the job creation process:
    /// - Fetching the next job ID from the queue.
    /// - Initialization of the job.
    /// - Status of queue and state updates.
    /// - Synchronization with the engine state after job creation.
    ///
    /// Ensure tracing is properly configured in your application to monitor these events.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn create_job(&mut self, s: &str) -> eyre::Result<Job> {
        // Fetch next ID
        //
        let nextid = call!(self.scheduler, |port| SchedulerMsg::Allocate(port))?;

        // Initialise job, list of task is empty
        //
        let job = JobBuilder::default().name(s.into()).id(nextid).build()?;

        // Update state
        //
        let _ = cast!(self.state, StateMsg::Add(nextid))?;

        trace!("job {} created.", nextid);
        self.sync()?;

        Ok(job)
    }

    /// Parses a job script string and creates a new job in the Ready state.
    ///
    /// This method takes a job script as input, parses it into a Job structure,
    /// and sets the job's state to Ready. The parsing process validates the script
    /// format and extracts necessary job configuration information.
    ///
    /// # Arguments
    ///
    /// * `job_script` - A string slice containing the job script to be parsed
    ///
    /// # Returns
    ///
    /// - On success, returns `Ok(Job)` containing the parsed and initialized job
    /// - On failure, returns an `Err` with details about what went wrong
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The job script contains invalid syntax
    /// - Required job parameters are missing
    /// - The parsing operation fails
    ///
    /// # Tracing
    ///
    /// This method is instrumented for tracing, excluding the `self` parameter.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn parse_job(&mut self, job_script: &str) -> Result<Job> {
        let mut job = self.parse(job_script).await?;
        job.set(JobState::Ready);
        Ok(job)
    }

    /// Submits a new job to be executed by parsing the job string, setting it to Ready state,
    /// and queuing it for execution.
    ///
    /// # Parameters
    ///
    /// - `job_str`: A string slice containing the job description to be parsed into a `Job`.
    ///
    /// # Returns
    ///
    /// - Returns `Ok(usize)` containing the ID of the newly created and queued job.
    /// - Returns `Err` if job creation, parsing, queueing or state sync fails.
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - Job string parsing fails
    /// - Job queueing fails
    /// - State synchronization fails
    ///
    /// # Notes
    ///
    /// The method performs the following steps:
    /// 1. Parses the job string into a Job struct
    /// 2. Sets the job state to Ready
    /// 3. Queues the job for execution
    /// 4. Synchronizes the engine state
    ///
    #[tracing::instrument(skip(self))]
    pub async fn submit_job(&mut self, job: Job) -> Result<WaitGroup> {
        if job.state() != JobState::Ready {
            return Err(EngineStatus::JobNotReady(job.id).into());
        }

        let (tx, rx) = channel::<Stats>();

        trace!("submit job {}", job.id);
        let wg = call!(self.scheduler, |port| { SchedulerMsg::Add(job, port) })?;

        // note will be for retrieving results later
        //
        Ok(wg)
    }

    #[tracing::instrument(skip(self))]
    pub async fn submit_job_and_wait(&mut self, job: Job) -> Result<Stats> {
        if job.state() != JobState::Ready {
            return Err(EngineStatus::JobNotReady(job.id).into());
        }

        trace!("submit job {}", job.id);
        let wg = call!(self.scheduler, |port| {
            SchedulerMsg::Add(job.clone(), port)
        })?;
        assert_eq!(wg.id, job.id);

        // Next tick, the job will run
        //
        trace!("wait for job {}", job.id);
        let stats = wg.rx.recv()?;
        trace!("job {} finished", job.id);

        let stats = call!(self.results, |port| ResultsMsg::Fetch(job.id, port))?;
        Ok(stats)
    }

    #[tracing::instrument(skip(self))]
    pub async fn wait_for(&mut self, id: usize, tx: Sender<Stats>) -> Result<Stats> {
        let res = call_t!(self.results, |port| ResultsMsg::Fetch(id, port), 10000)?;

        tx.send(res.clone())?;
        Ok(res)
    }

    #[tracing::instrument(skip(self))]
    pub fn shutdown(&mut self) {
        pg::get_members(&ENGINE_PG.to_string())
            .iter()
            .for_each(|cell| {
                cell.stop(Some("ctrl-C pressed".into()));
            });
    }

    #[tracing::instrument(skip(self))]
    pub fn ps(&mut self) {
        let v = self.version();
        eprintln!("Engin version {} is running", self.version());

        let plist = registered().join("\n");
        eprintln!("Actor list:\n{plist}");
    }

    /// Removes a job from the engine by its ID.
    ///
    /// This method attempts to remove a job with the specified ID from the engine's job queue.
    /// The job cannot be removed if it is currently in the Running state.
    ///
    /// # Arguments
    ///
    /// - `job_id`: The unique identifier of the job to remove.
    ///
    /// # Returns
    ///
    /// - On success, returns `Ok(())` after removing the job and syncing state.
    /// - On failure, returns an `Err` containing details about what went wrong.
    ///
    /// # Errors
    ///
    /// This method will return an error in the following cases:
    ///
    /// - If the job is currently running (`EngineStatus::JobIsRunning`)
    /// - If the job cannot be found in the queue
    /// - If state synchronization fails after removal
    ///
    /// # Tracing
    ///
    /// This method is instrumented for tracing, excluding the `self` parameter.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn remove_job(&mut self, job_id: usize) -> Result<()> {
        let _ = cast!(self.state, StateMsg::Remove(job_id))?;
        let _ = cast!(self.scheduler, SchedulerMsg::RemoveById(job_id))?;
        self.sync()
    }
}

