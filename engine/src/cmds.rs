use crate::actors::{QueueMsg, StateMsg};
use crate::{Engine, EngineStatus, Job, JobState};
use ractor::{call, cast};
use tracing::{error, trace};

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
        let nextid = call!(self.queue, |port| QueueMsg::Allocate(port))?;

        // Initialise job, list of task is empty
        //
        let job = Job::new(s, nextid);

        // Update state
        //
        let _ = cast!(self.state, StateMsg::Add(nextid))?;

        trace!("job {} created.", nextid);
        self.sync()?;

        Ok(job)
    }

    /// Queue a job for execution in the engine.
    ///
    /// This method takes a job that is in the "Ready" state and queues it for execution
    /// by changing its state to "Queued" and adding it to the engine's job queue.
    ///
    /// # Arguments
    ///
    /// * `job` - The `Job` instance to be queued. The job must be in the `Ready` state.
    ///
    /// # Returns
    ///
    /// - On success, returns `Ok(usize)` containing the job's ID
    /// - On failure, returns an `Err` containing details about what went wrong
    ///
    /// # Errors
    ///
    /// This method will return an error in the following cases:
    ///
    /// - If the job is not in the `Ready` state (returns `EngineStatus::JobNotReady`)
    /// - If adding the job to the queue fails
    ///
    /// # Tracing
    ///
    /// This method is instrumented for tracing, excluding the `self` parameter.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn queue_job(&mut self, job: Job) -> eyre::Result<usize> {
        if job.state != JobState::Ready {
            error!("Job is not ready");
            return Err(EngineStatus::JobNotReady(job.id).into());
        }

        // Change status and insert the job into the queue.
        //
        let mut ready = job.clone();
        ready.state = JobState::Queued;
        let _ = cast!(self.queue, QueueMsg::Add(ready))?;
        Ok(job.id)
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
    pub async fn remove_job(&mut self, job_id: usize) -> eyre::Result<()> {
        let job = call!(self.queue, |port| QueueMsg::GetById(job_id, port))?;
        if job.state == JobState::Running {
            return Err(EngineStatus::JobIsRunning(job_id).into());
        }

        let _ = cast!(self.state, StateMsg::Remove(job_id))?;
        let _ = cast!(self.queue, QueueMsg::RemoveById(job_id))?;
        self.sync()
    }

    /// Retrieve a job by its unique ID
    ///
    /// This method takes a job ID (of type `usize`) and attempts to retrieve the
    /// corresponding job from the internal job queue. If a job with the specified ID
    /// exists, it is returned; otherwise, an error is generated indicating the job
    /// could not be found.
    ///
    /// # Arguments
    ///
    /// - `id`: A `usize` identifier representing the unique ID of the job to retrieve.
    ///
    /// # Returns
    ///
    /// - Returns the `Job` instance if it exists.
    /// - Returns an error if the job with the specified ID is not found.
    ///
    /// # Errors
    ///
    /// This method will return an `Err` containing `EngineStatus::JobNotFound` if
    /// the job does not exist in the internal job queue.
    ///
    /// # Tracing
    ///
    /// Tracing logs are emitted to provide detailed runtime diagnostics, including:
    /// - Lock acquisition on the job list.
    /// - Retrieval success or error cases.
    ///
    /// Ensure tracing is set up in your application to observe these events.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn get_job(&self, id: usize) -> eyre::Result<Job> {
        let job = call!(self.queue, |port| QueueMsg::GetById(id, port))?;

        Ok(job.clone())
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
    /// This method will return an error if:
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
    pub async fn submit_job(&mut self, job_str: &str) -> eyre::Result<usize> {
        let mut job = self.parse(job_str).await?;
        job.state = JobState::Ready;

        let job_id = self.queue_job(job.clone()).await?;
        assert_eq!(job_id, job.id);
        self.sync()?;
        Ok(job_id)
    }
}