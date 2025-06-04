//! Main single mode engine instantiation
//!
use std::path::PathBuf;

use crate::JobText;
use eyre::Result;
pub use fetiche_engine::{Engine, Job, Stats};
use tracing::debug;

#[derive(Clone, Debug)]
pub struct EngineSingle {
    /// The engine instance
    e: Engine,
}

impl EngineSingle {
    #[tracing::instrument]
    pub async fn new() -> Result<Self> {
        Ok(Self {
            e: Engine::single().await?,
        })
    }

    // ----- wrappers

    #[tracing::instrument(skip(self))]
    pub async fn create_job(&mut self, job: &str) -> Result<Job> {
        self.e.create_job(job).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn parse_job(&mut self, job: JobText) -> Result<Job> {
        let job = hcl::to_string(&job)?;
        debug!("jobtext = {}", job);
        self.e.parse_job(&job).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn submit_job_and_wait(&mut self, job: Job) -> Result<Stats> {
        self.e.submit_job_and_wait(job).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn cleanup(&mut self) -> Result<()> {
        self.e.cleanup().await
    }

    #[tracing::instrument(skip(self))]
    pub fn shutdown(&mut self) {
        self.e.shutdown();
    }

    // ----- Misc. wrappers

    #[tracing::instrument(skip(self))]
    pub fn config_file(&self) -> PathBuf {
        self.e.config_file()
    }

    #[tracing::instrument(skip(self))]
    pub fn sources_file(&self) -> PathBuf {
        self.e.sources_file()
    }

    #[tracing::instrument(skip(self))]
    pub fn list_containers(&mut self) -> Result<String> {
        self.e.list_containers()
    }

    #[tracing::instrument(skip(self))]
    pub fn list_commands(&mut self) -> Result<String> {
        self.e.list_commands()
    }

    #[tracing::instrument(skip(self))]
    pub fn list_formats(&mut self) -> Result<String> {
        self.e.list_formats()
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_sources(&mut self) -> Result<String> {
        self.e.list_sources().await
    }

    #[tracing::instrument(skip(self))]
    pub fn list_storage(&mut self) -> Result<String> {
        self.e.list_storage()
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_tokens(&mut self) -> Result<String> {
        self.e.list_tokens().await
    }

    #[tracing::instrument(skip(self))]
    fn inner(&mut self) -> &mut Engine {
        &mut self.e
    }

    #[tracing::instrument(skip(self))]
    pub fn version(&mut self) -> String {
        self.e.version()
    }
}
