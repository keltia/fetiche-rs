//! Misc. utility members for `Engine`.
//!

use std::sync::Arc;

use eyre::Result;

use crate::{version, Engine, Storage};
use fetiche_common::Container;
use fetiche_formats::Format;
use fetiche_sources::Sources;

impl Engine {
    /// Return an `Arc::clone` of the Engine sources
    ///
    pub fn sources(&self) -> Arc<Sources> {
        Arc::clone(&self.sources)
    }

    /// Return an `Arc::clone` of the Engine storage areas
    ///
    pub fn storage(&self) -> Arc<Storage> {
        Arc::clone(&self.storage)
    }

    /// Returns a list of all defined storage areas
    ///
    pub fn list_storage(&self) -> Result<String> {
        self.storage.list()
    }

    /// Return a description of all supported sources
    ///
    pub fn list_sources(&self) -> Result<String> {
        self.sources.list()
    }

    /// Return a descriptions of all supported data formats
    ///
    pub fn list_formats(&self) -> Result<String> {
        Format::list()
    }

    /// Return a descriptions of all supported container formats
    ///
    pub fn list_containers(&self) -> Result<String> {
        Container::list()
    }

    /// Return a list of all currently available authentication tokens
    ///
    pub fn list_tokens(&self) -> Result<String> {
        self.tokens.list()
    }

    /// Return Engine version (and internal modules)
    ///
    pub fn version(&self) -> String {
        format!(
            "{} ({} {} {})",
            version(),
            fetiche_formats::version(),
            fetiche_sources::version(),
            fetiche_common::version(),
        )
    }
}
