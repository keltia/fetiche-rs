//! Main single mode engine instantiation
//!
use std::path::PathBuf;

use eyre::Result;
use tracing::debug;

use crate::{JobText, Workspace};
pub use fetiche_engine::{Engine, Job, Stats};

#[derive(Clone, Debug)]
pub struct EngineSingle {
    /// The engine instance
    e: Engine,
}

impl EngineSingle {
    #[tracing::instrument]
    pub async fn new(ws: &Workspace) -> Result<Self> {
        // Any `Engine` instance should be within a `Workspace`.
        //
        let basedir = ws.path();
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
    pub async fn shutdown(&mut self) -> Result<()> {
        Ok(self.e.shutdown().await?)
    }

    // ----- Misc. wrappers

    #[tracing::instrument(skip(self))]
    pub fn config_file(&self) -> Result<PathBuf> {
        self.e.config_file()
    }

    #[tracing::instrument(skip(self))]
    pub fn sources_file(&self) -> Result<PathBuf> {
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
    pub fn version(&mut self) -> String {
        self.e.version()
    }

    #[tracing::instrument(skip(self))]
    pub fn ws(&mut self) -> Workspace {
        self.e.ws()
    }

    #[tracing::instrument(skip(self))]
    pub fn inner(&mut self) -> &Engine {
        &self.e
    }
}
