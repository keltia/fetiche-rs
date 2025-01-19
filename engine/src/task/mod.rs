//! Regroup all available task/commands
//!

use std::collections::BTreeMap;

use enum_dispatch::enum_dispatch;
use eyre::Result;
use serde::Deserialize;
use tabled::{builder::Builder, settings::Style};
use tracing::trace;

pub use archive::*;
pub use common::*;
pub use convert::*;
pub use fetch::*;
pub use read::*;
pub use save::*;
pub use store::*;
pub use stream::*;
pub use tee::*;

use crate::{Engine, IO};

mod archive;
mod common;
mod convert;
mod fetch;
mod read;
mod save;
mod store;
mod stream;
mod tee;

/// The `Task` enum represents all available tasks/commands that can be used.
/// Each variant corresponds to a specific operation or process.
///
/// Variants:
///
/// - `Archive`: Extract streaming data and generate `CSV` or `Parquet`.
/// - `Convert`: Convert between different `Format` representations.
/// - `Copy`: Perform a basic raw copy operation.
/// - `Fetch`: Fetch a single dataset from a specific source.
/// - `Message`: Display a message.
/// - `Nothing`: Represents a no-operation (NOP) task.
/// - `Read`: Read data from a single file.
/// - `Save`: Save a single dataset to a specific destination.
/// - `Store`: Organize and store datasets into a directory structure.
/// - `Stream`: Fetch and process a continuous stream of data.
/// - `Tee`: Copy data into a file and pass the data forward unchanged.
#[enum_dispatch]
#[derive(Clone, Debug, strum::Display, strum::VariantNames)]
#[strum(serialize_all = "PascalCase")]
pub enum Task {
    /// Extract streaming data and generate csv/parquet.
    Archive,
    /// Convert between `Format`
    Convert,
    /// Basic raw copy
    Copy,
    /// Fetch a single dataset
    Fetch,
    /// Display a message
    Message,
    /// NOP
    Nothing,
    /// Read a single file
    Read,
    /// Save a single dataset
    Save,
    /// Store datasets into a organised directory
    Store,
    /// Fetch a stream of data
    Stream,
    /// Copy data and pass it along
    Tee,
}

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
    pub fn list_commands(&self) -> Result<String> {
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
