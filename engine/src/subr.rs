//! Miscellaneous utilities members for `Engine`.
//!
//! Most functions will be just wrappers for the messages to the `SourcesActor`.
//!

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use eyre::Result;
use object_store::path::Path;
use ractor::{call, cast};
use serde::Deserialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::trace;

use fetiche_common::Container;
use fetiche_formats::Format;

use crate::actors::{SourcesMsg, StateMsg};
use crate::{version, Engine, Sources, Storage, ENGINE_CONFIG, IO, SOURCES_CONFIG, STATE_FILE};

impl Engine {
    /// Returns the path of the default state file in the engine's base directory
    ///
    /// This method combines the engine's home directory with the STATE_FILE constant
    /// to provide the full path where state information should be persisted.
    ///
    #[inline]
    pub fn state_file(&self) -> Result<PathBuf> {
        Ok(self.home
            .path_to_filesystem(&Path::from(STATE_FILE))?)
    }

    #[inline]
    pub fn sources_file(&self) -> Result<PathBuf> {
        Ok(self.home
            .path_to_filesystem(&Path::from(SOURCES_CONFIG))?)
    }

    /// Synchronizes all engine state by persisting it to disk
    ///
    /// This method sends a sync message to the state actor which handles
    /// writing the current state to the configured state file location.
    ///
    #[tracing::instrument(skip(self))]
    #[inline]
    pub fn sync(&self) -> Result<()> {
        trace!("engine::sync");

        Ok(cast!(self.state, StateMsg::Sync)?)
    }

    /// Returns a copy of all configured Engine sources
    ///
    /// This asynchronous method requests the current list of sources from the sources actor,
    /// which maintains the authoritative list of all configured data sources.
    ///
    pub async fn sources(&self) -> Result<Sources> {
        let src = call!(self.sources, SourcesMsg::List)?;
        Ok(src)
    }

    /// Returns a thread-safe reference-counted clone of the Engine storage configuration
    ///
    /// This provides access to the storage areas configuration while maintaining
    /// proper reference counting through Arc.
    ///
    pub fn storage(&self) -> Arc<Storage> {
        Arc::clone(&self.storage)
    }

    /// Returns a formatted string containing all defined storage areas
    ///
    /// The returned string contains a human-readable list of all configured
    /// storage locations and their properties.
    ///
    pub fn list_storage(&self) -> Result<String> {
        self.storage.list()
    }

    /// Returns a formatted table describing all supported data sources
    ///
    /// This asynchronous method generates a human-readable table containing
    /// details about each configured data source and its capabilities.
    ///
    pub async fn list_sources(&self) -> Result<String> {
        let src = call!(self.sources, SourcesMsg::Table)?;
        Ok(src)
    }

    /// Returns a formatted list of all supported data formats
    ///
    /// Provides information about which data formats the engine can process,
    /// including their names and characteristics.
    ///
    pub fn list_formats(&self) -> Result<String> {
        Format::list()
    }

    /// Returns a formatted list of all supported container formats
    ///
    /// Lists all container formats that can be used to package and
    /// transport data within the engine.
    ///
    pub fn list_containers(&self) -> Result<String> {
        Container::list()
    }

    /// Returns a formatted list of all currently available authentication tokens
    ///
    /// Provides information about active authentication tokens used for
    /// accessing various data sources.
    ///
    pub async fn list_tokens(&self) -> Result<String> {
        self.tokens.as_string().await
    }

    /// Returns the full path to the engine's configuration file
    ///
    /// Combines the engine's home directory with the ENGINE_CONFIG constant
    /// to locate the HCL configuration file.
    ///
    pub fn config_file(&self) -> Result<PathBuf> {
        Ok(self.home
            .path_to_filesystem(&Path::from(ENGINE_CONFIG))?)
    }

    /// Returns a string containing version information for the engine and its modules
    ///
    /// Formats a string that includes the engine's version number along with
    /// version information for the formats and common modules.
    ///
    pub fn version(&self) -> String {
        format!(
            "{} ({} {})",
            version(),
            fetiche_formats::version(),
            fetiche_common::version(),
        )
    }
}

// -----

/// For each format, we define a set of key attributes that will get displayed.
///
#[derive(Debug, Deserialize)]
pub struct CmdsDescr {
    /// Type of data each command refers to
    #[serde(rename = "type")]
    pub ctype: IO,
    /// Free text description
    pub description: String,
}

/// Current version of the cmds.hcl file.
const CVERSION: usize = 1;

/// Struct to be read from an HCL file at compile-time
///
#[derive(Debug, Deserialize)]
pub struct CmdsFile {
    /// Version
    pub version: usize,
    /// Ordered list of format metadata
    pub cmds: BTreeMap<String, CmdsDescr>,
}

impl Engine {
    /// Returns a formatted table of all available commands from the cmds.hcl file
    ///
    /// Parses the embedded cmds.hcl file and creates a formatted table showing
    /// each command's name, type, and description. Validates the file version
    /// before processing.
    ///
    #[tracing::instrument]
    pub fn list_commands(&self) -> eyre::Result<String> {
        trace!("list all commands");

        let allcmds_s = include_str!("cmds.hcl");
        let allcmds: CmdsFile = hcl::from_str(allcmds_s)?;

        // Safety checks
        //
        assert_eq!(allcmds.version, CVERSION);

        let header = vec!["Name", "Type", "Description"];

        let mut builder = Builder::default();
        builder.push_record(header);

        allcmds
            .cmds
            .iter()
            .for_each(|(cmd, cmd_desc): (&String, &CmdsDescr)| {
                let mut row = vec![];

                let name = cmd.clone();
                let ctype = cmd_desc.ctype.clone().to_string();
                let descr = cmd_desc.description.clone();
                row.push(name);
                row.push(ctype);
                row.push(descr);
                builder.push_record(row);
            });

        let allc = builder.build().with(Style::modern()).to_string();
        let str = format!("List all commands:\n{allc}");

        Ok(str)
    }
}
