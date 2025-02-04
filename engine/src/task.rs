//! Regroup all available task/commands
//!

use std::collections::BTreeMap;

use enum_dispatch::enum_dispatch;
use eyre::Result;
use serde::Deserialize;
use strum::EnumString;
use tabled::{builder::Builder, settings::Style};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tracing::trace;

use crate::{Consumer, Engine, EngineStatus, Middle, Pipeline, Producer, Runnable};

/// Task I/O characteristics
///
/// The main principle being that a consumer should not be first in a job queue
/// just like a producer one should not be last.
///
#[derive(Clone, Debug, Default, Eq, PartialEq, EnumString, strum::Display, Deserialize)]
#[strum(serialize_all = "PascalCase")]
pub enum IO {
    /// Consumer (no output or different like file)
    Consumer,
    /// Producer (discard input)
    Producer,
    /// Both (filter)
    #[default]
    Filter,
    /// Cache (filter)
    Cache,
}

/// Task represents different types of tasks that can be performed in the data processing pipeline.
///
/// Each variant corresponds to a different stage in the pipeline:
/// - Producer: Tasks that generate or source data
/// - Middle: Tasks that transform or process data
/// - Consumer: Tasks that consume or store the final data
///
#[derive(Clone, Debug)]
pub enum Task {
    /// Producer task that generates or sources data
    Producer(Producer),
    /// Middle task that transforms or processes data
    Middle(Middle),
    /// Consumer task that consumes or stores the final data
    Consumer(Consumer),
}

// impl Task {
//     pub fn run(&self, rec: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>) {
//         let (tx, rx) = std::sync::mpsc::channel::<String>();
//         match self {
//             // Producer doesn't care about the input data, it generates it
//             //
//             Task::Producer(mut p) => {
//                 let (rx, h) = p.run(rec);
//                 (rx, h)
//             }
//             Task::Middle(mut m) => {
//                 let (rx, h) = m.run(rec);
//                 (rx, h)
//             }
//             Task::Consumer(mut c) => {
//                 let (rx, h) = c.run(rec);
//                 (rx, h)
//             }
//         }
//     }
// }

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
