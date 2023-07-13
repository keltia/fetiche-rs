use std::collections::BTreeMap;

use anyhow::Result;
use serde::Deserialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::trace;

pub use influxdb::*;
pub use mysql::*;

use crate::Engine;

mod influxdb;
mod mysql;

/// All supported databases
///
#[derive(Debug, Default)]
pub enum DB {
    #[default]
    InfluxDB,
    Mysql,
    Pgsql,
}

/// For each format, we define a set of key attributes that will get displayed.
///
#[derive(Debug, Deserialize)]
pub struct DatabaseDescr {
    /// Type of data each command refers to
    #[serde(rename = "type")]
    pub dtype: DB,
    /// Connection URL
    pub url: String,
    /// Free text description
    pub description: String,
}

/// Current version of the cmds.hcl file.
const DVERSION: usize = 1;

/// Struct to be read from an HCL file at compile-time
///
#[derive(Debug, Deserialize)]
pub struct DatabaseFile {
    /// Version
    pub version: usize,
    /// Ordered list of format metadata
    pub cmds: BTreeMap<String, DatabaseDescr>,
}

impl Engine {
    /// Returns the content of the `cmds.hcl` file as a table.
    ///
    #[tracing::instrument]
    pub fn list_commands(&self) -> Result<String> {
        trace!("list all commands");

        let data = include_str!("cmds.hcl");
        let dbs: DatabaseDescr = hcl::from_str(data)?;

        // Safety checks
        //
        assert_eq!(dbs.version, CVERSION);

        let header = vec!["Name", "Type", "Description"];

        let mut builder = Builder::default();
        builder.set_header(header);

        dbs.cmds
            .iter()
            .for_each(|(cmd, cmd_desc): (&String, &DatabaseDescr)| {
                let mut row = vec![];

                let name = cmd.clone();
                let ctype = cmd_desc.dtype.clone().to_string();
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
