//! Main single mode engine instantiation
//!
use std::path::PathBuf;

use crate::JobText;
use eyre::Result;
pub use fetiche_engine::{Engine, Job, Stats};

/// Client for configuring and running jobs with a local engine in single mode.
///
#[derive(Clone, Debug)]
pub struct EngineSingle {
    /// The engine instance
    ///
    e: Engine,
}

impl EngineSingle {
    /// Starts an in-process engine using the standard `engine.hcl` configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be loaded or the engine
    /// cannot initialize its workspace, storage, or actors.
    ///
    #[tracing::instrument]
    pub async fn new() -> Result<Self> {
        Ok(Self {
            e: Engine::single().await?,
        })
    }

    // ----- wrappers

    /// Allocates a job ID and registers an empty job with the given name.
    ///
    /// Use [`Self::parse_job`] to create a job with a configured pipeline.
    ///
    /// # Errors
    ///
    /// Returns an error if ID allocation, state registration, or state
    /// synchronization fails.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn create_job(&mut self, job: &str) -> Result<Job> {
        self.e.create_job(job).await
    }

    /// Serializes the job definition to HCL and returns a [`JobState::Ready`](crate::JobState::Ready) job.
    ///
    /// The engine resolves configured source names and builds the pipeline.
    /// Call [`Self::submit_job_and_wait`] to run it.
    ///
    /// # Errors
    ///
    /// Returns an error if HCL serialization or parsing fails, a source cannot
    /// be resolved or lacks the requested capability, or job setup fails.
    ///
    /// # Panics
    ///
    /// The engine parser currently panics on [`MiddleText::Convert`](crate::MiddleText::Convert).
    ///
    #[tracing::instrument(skip(self, job))]
    pub async fn parse_job(&mut self, job: JobText) -> Result<Job> {
        let job = hcl::to_string(&job)?;
        self.e.parse_job(&job).await
    }

    /// Submits a ready job and returns its final [`Stats`] after it finishes.
    ///
    /// This call blocks while waiting for completion. The engine changes the
    /// process working directory to its workspace and restores it on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the job is not ready, the engine cannot change
    /// directories, scheduling fails, or the completion signal or statistics
    /// cannot be retrieved.
    ///
    #[tracing::instrument(skip(self, job), fields(job_id = job.id))]
    pub async fn submit_job_and_wait(&mut self, job: Job) -> Result<Stats> {
        self.e.submit_job_and_wait(job).await
    }

    /// Attempts to remove the single-mode engine's working directory and its contents.
    ///
    /// The engine ignores directory removal errors. Call [`Self::shutdown`]
    /// before cleanup if you also need to stop the engine and remove its run marker.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn cleanup(&mut self) -> Result<()> {
        self.e.cleanup().await
    }

    /// Requests that the engine actors stop and removes the workspace's run marker.
    ///
    /// # Errors
    ///
    /// Returns an error if the run marker cannot be removed.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn shutdown(&mut self) -> Result<()> {
        Ok(self.e.shutdown().await?)
    }

    // ----- Misc. wrappers

    /// Returns the filesystem path to `engine.hcl` under the engine's configuration directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine cannot resolve the filesystem path.
    ///
    #[tracing::instrument(skip(self))]
    pub fn config_file(&self) -> Result<PathBuf> {
        self.e.config_file()
    }

    /// Returns the filesystem path to the data source configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine cannot resolve the filesystem path.
    ///
    #[tracing::instrument(skip(self))]
    pub fn sources_file(&self) -> Result<PathBuf> {
        self.e.sources_file()
    }

    /// Returns a human-readable list of supported container formats.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded container definitions are invalid
    /// or use an unexpected schema version.
    ///
    #[tracing::instrument(skip(self))]
    pub fn list_containers(&mut self) -> Result<String> {
        self.e.list_containers()
    }

    /// Returns a human-readable table of command names, types, and descriptions.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded command definitions cannot be parsed.
    ///
    /// # Panics
    ///
    /// Panics if the embedded definitions use an unexpected schema version.
    ///
    #[tracing::instrument(skip(self))]
    pub fn list_commands(&mut self) -> Result<String> {
        self.e.list_commands()
    }

    /// Returns a human-readable list of supported data formats.
    ///
    /// # Errors
    ///
    /// Returns an error if the embedded format definitions cannot be parsed.
    ///
    /// # Panics
    ///
    /// Panics if the embedded definitions use an unexpected schema version.
    ///
    #[tracing::instrument(skip(self))]
    pub fn list_formats(&mut self) -> Result<String> {
        self.e.list_formats()
    }

    /// Returns a human-readable table of configured data sources and their capabilities.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine cannot obtain the table from its sources actor.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn list_sources(&mut self) -> Result<String> {
        self.e.list_sources().await
    }

    /// Returns a human-readable table of configured storage areas, their paths
    /// or URLs, and their rotation settings.
    ///
    #[tracing::instrument(skip(self))]
    pub fn list_storage(&mut self) -> Result<String> {
        self.e.list_storage()
    }

    /// Returns a human-readable table of stored authentication token metadata,
    /// including filenames, producer labels, and creation times.
    ///
    /// # Errors
    ///
    /// Returns an error if the token store cannot be listed.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn list_tokens(&mut self) -> Result<String> {
        self.e.list_tokens().await
    }

    /// Returns the engine, formats, and common crate version strings.
    ///
    /// See [`crate::version`] for the client crate's version.
    ///
    #[tracing::instrument(skip(self))]
    pub fn version(&mut self) -> String {
        self.e.version()
    }

    /// Borrows the underlying [`Engine`] for access to its shared-reference API.
    ///
    #[tracing::instrument(skip(self))]
    pub fn inner(&mut self) -> &Engine {
        &self.e
    }
}
