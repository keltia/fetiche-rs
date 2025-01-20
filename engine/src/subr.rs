//! Misc. utility members for `Engine`.
//!
//! Most functions will be just wrappers for the messages to the `SourcesActor`.
//!

use std::path::PathBuf;
use std::sync::Arc;

use eyre::Result;
use ractor::{call, cast};
use tracing::trace;

use fetiche_common::Container;
use fetiche_formats::Format;
use fetiche_sources::Sources;

use crate::actors::{SourcesMsg, StateMsg};
use crate::{version, Engine, Storage, STATE_FILE};

impl Engine {
    /// Returns the path of the default state file in basedir
    ///
    #[inline]
    pub fn state_file(&self) -> PathBuf {
        self.home.join(STATE_FILE)
    }

    /// Sync all state into a file
    ///
    #[tracing::instrument(skip(self))]
    #[inline]
    pub fn sync(&self) -> Result<()> {
        trace!("engine::sync");

        Ok(cast!(self.state, StateMsg::Sync)?)
    }

    /// Return a copy of the Engine sources
    ///
    pub async fn sources(&self) -> Result<Sources> {
        let src = call!(self.sources, |port| SourcesMsg::List(port))?;
        Ok(src)
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
    pub async fn list_sources(&self) -> Result<String> {
        let src = call!(self.sources, |port| SourcesMsg::Table(port))?;
        Ok(src)
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
