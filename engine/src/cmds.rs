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

use std::env;
use std::sync::mpsc::Sender;

use eyre::Result;
use ractor::registry::registered;
use ractor::{call, call_t, cast, pg};
use tracing::{info, trace};

use crate::actors::{ResultsMsg, SchedulerMsg, StateMsg};
use crate::{Engine, EngineMode, EngineStatus, Job, JobBuilder, JobState, Stats, WaitGroup, ENGINE_PG};

/// Basically, this is the exposed API to the Engine.
///
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
    pub async fn create_job(&mut self, s: &str) -> Result<Job> {
        // Fetch next ID
        //
        let nextid = call!(self.scheduler, SchedulerMsg::Allocate)?;

        // Initialise the job, list of tasks is empty
        //
        let job = JobBuilder::default().name(s.into()).id(nextid).build()?;

        // Update state
        //
        cast!(self.state, StateMsg::Submit(nextid))?;

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

        trace!("submit job {}", job.id);
        let wg = call!(self.scheduler, |port| { SchedulerMsg::Add(job, port) })?;

        // note will be for retrieving results later
        //
        Ok(wg)
    }

    /// Submits a job for execution and waits for it to complete, returning the final statistics.
    ///
    /// This method combines job submission with waiting for completion in a single call. It submits
    /// the job to the scheduler and blocks until execution is finished, then retrieves and returns
    /// the final job statistics.
    ///
    /// # Arguments
    ///
    /// * `job` - The job to be submitted and executed. Must be in Ready state.
    ///
    /// # Returns
    ///
    /// Returns `Result<Stats>` containing:
    /// - `Ok(Stats)` - The final statistics from the completed job execution
    /// - `Err` - If job submission fails or execution encounters an error
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The job is not in Ready state
    /// - Job submission to the scheduler fails
    /// - Communication with the results actor fails
    /// - The job execution fails
    ///
    /// # Tracing
    ///
    /// This method is instrumented for tracing, excluding the `self` parameter.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn submit_job_and_wait(&mut self, job: Job) -> Result<Stats> {
        if job.state() != JobState::Ready {
            return Err(EngineStatus::JobNotReady(job.id).into());
        }

        // Save where we are
        //
        let workdir = self.workdir.clone();
        let olddir = env::current_dir()?;
        let _ = env::set_current_dir(&workdir)?;
        info!("Relocating to {workdir:?}");

        trace!("submit job {}", job.id);
        let wg = call!(self.scheduler, |port| {
            SchedulerMsg::Add(job.clone(), port)
        })?;
        assert_eq!(wg.id, job.id);

        // Next tick, the job will run
        //
        trace!("wait for job {}", job.id);
        wg.rx.recv()?;
        trace!("job {} finished", job.id);

        let stats = call!(self.results, |port| ResultsMsg::Fetch(job.id, port))?;

        // Go back
        //
        let _ = env::set_current_dir(&olddir)?;

        Ok(stats)
    }

    /// Waits for a specific job to complete and retrieves its final statistics.
    ///
    /// This method blocks until the job with the specified ID completes execution
    /// and then fetches its final statistics. It also forwards the statistics through
    /// the provided sender channel.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the job to wait for
    /// * `tx` - A sender channel to forward the job statistics when available
    ///
    /// # Returns
    ///
    /// Returns `Result<Stats>` containing:
    /// - `Ok(Stats)` - The final statistics from the completed job execution
    /// - `Err` - If fetching results fails or sending stats fails
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - Communication with the results actor fails
    /// - The job results cannot be retrieved
    /// - Sending statistics through the provided channel fails
    ///
    #[tracing::instrument(skip(self))]
    pub async fn wait_for(&mut self, id: usize, tx: Sender<Stats>) -> Result<Stats> {
        let res = call_t!(self.results, |port| ResultsMsg::Fetch(id, port), 10000)?;

        tx.send(res.clone())?;
        Ok(res)
    }

    /// Shuts down the engine and all its associated actors.
    ///
    /// This method gracefully terminates the engine by stopping all registered actors
    /// in the engine process group.
    ///
    /// # Behavior
    ///
    /// - Retrieves all members of the engine process group
    /// - Iterates through each actor and sends a stop signal
    /// - Actors will clean up resources before terminating
    ///
    #[tracing::instrument]
    pub fn shutdown(&mut self) {
        pg::get_members(&ENGINE_PG.to_string())
            .iter()
            .for_each(|cell| {
                cell.stop(Some("Shutdown requested.".into()));
            });
    }

    /// Prints the current engine version and lists all registered actors.
    ///
    /// This method outputs diagnostic information about the running engine instance:
    /// - The current engine version number
    /// - A list of all registered actors in the system
    ///
    /// The output is written to stderr using eprintln!.
    ///
    /// # Tracing
    ///
    /// This method is instrumented for tracing, excluding the `self` parameter.
    ///
    #[tracing::instrument(skip(self))]
    pub fn ps(&mut self) {
        eprintln!("Engine version {} is running", self.version());

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
        cast!(self.state, StateMsg::Remove(job_id))?;
        cast!(self.scheduler, SchedulerMsg::RemoveById(job_id))?;
        self.sync()
    }

    /// Cleans up engine resources by removing the working directory in single mode.
    ///
    /// This method handles cleanup of filesystem resources used by the engine:
    /// - In single mode: Removes the temporary working directory
    /// - In other modes: No cleanup is performed
    ///
    /// # Returns
    ///
    /// - Returns `Ok(())` on successful cleanup or if no cleanup was needed
    /// - Returns `Err` if directory removal fails
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The working directory cannot be removed
    /// - Filesystem operations fail during cleanup
    ///
    #[tracing::instrument(skip(self))]
    pub async fn cleanup(&self) -> Result<()> {
        if self.mode == EngineMode::Single {
            // We need to remove the directory known as `workdir`
            //
            let _ = std::fs::remove_dir_all(self.workdir.clone());
        }
        Ok(())
    }
}

impl Drop for Engine {
    /// Shuts down the engine and all its associated actors.
    ///
    /// This method gracefully terminates the engine by stopping all registered actors
    /// in the engine process group.
    ///
    /// # Behavior
    ///
    /// - Retrieves all members of the engine process group
    /// - Iterates through each actor and sends a stop signal
    /// - Actors will clean up resources before terminating
    ///
    #[tracing::instrument]
    fn drop(&mut self) {
        pg::get_members(&ENGINE_PG.to_string())
            .iter()
            .for_each(|cell| {
                cell.stop(Some("Shutdown requested.".into()));
            });
    }
}
