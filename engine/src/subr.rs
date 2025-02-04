//! Miscellaneous utilities members for `Engine`.
//!
//! Most functions will be just wrappers for the messages to the `SourcesActor`.
//!

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use eyre::Result;
use ractor::{call, cast};
use serde::Deserialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::trace;

use fetiche_common::Container;
use fetiche_formats::Format;

use crate::actors::{SourcesMsg, StateMsg};
use crate::{version, Engine, Sources, Storage, ENGINE_CONFIG, IO, STATE_FILE};

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
        self.tokens.to_string()
    }

    /// Return the path of the `engine.hcl` file.
    ///
    pub fn config_file(&self) -> PathBuf {
        self.home.join(ENGINE_CONFIG)
    }

    /// Return Engine version (and internal modules)
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
    /// Returns the content of the `cmds.hcl` file as a table.
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
