use std::collections::BTreeMap;

use eyre::Result;
use serde::Deserialize;
use strum::{EnumIter, EnumString};
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
#[derive(
    Clone, Debug, Default, Deserialize, strum::Display, strum::VariantNames, EnumIter, EnumString,
)]
#[strum(serialize_all = "PascalCase")]
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
    pub db: BTreeMap<String, DatabaseDescr>,
}

impl Engine {
    /// Returns the content of the `cmds.hcl` file as a table.
    ///
    #[tracing::instrument]
    pub fn list_databases(&self) -> Result<String> {
        trace!("list all databases");

        let data = include_str!("databases.hcl");
        let dbs: DatabaseFile = hcl::from_str(data)?;

        // Safety checks
        //
        assert_eq!(dbs.version, DVERSION);

        let header = vec!["Name", "Type", "URL", "Description"];

        let mut builder = Builder::default();
        builder.push_record(header);

        dbs.db
            .iter()
            .for_each(|(db, db_desc): (&String, &DatabaseDescr)| {
                let mut row = vec![];

                let name = db.clone();
                let dtype = db_desc.dtype.clone().to_string();
                let url = db_desc.url.clone();
                let descr = db_desc.description.clone();
                row.push(name);
                row.push(dtype);
                row.push(url);
                row.push(descr);
                builder.push_record(row);
            });

        let allc = builder.build().with(Style::modern()).to_string();
        let str = format!("List all databases:\n{allc}");

        Ok(str)
    }
}
