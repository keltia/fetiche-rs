//! Main single mode engine instantiation
//!
use fetiche_engine::Engine;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct EngineSingle {
    /// The engine instance
    e: Engine,
}

impl EngineSingle {
    #[tracing::instrument]
    pub async fn new() -> Self {
        Self {
            e: Engine::single().await,
        }
    }

    // ----- wrappers

    #[tracing::instrument(skip(self))]
    pub fn shutdown(&mut self) {
        self.e.shutdown();
    }

    #[tracing::instrument(skip(self))]
    pub fn config_file(&self) -> PathBuf {
        self.e.config_file()
    }

    #[tracing::instrument(skip(self))]
    pub fn sources_file(&self) -> PathBuf {
        self.e.sources_file()
    }

    #[tracing::instrument(skip(self))]
    pub fn inner(&mut self) -> &mut Engine {
        &mut self.e
    }
}