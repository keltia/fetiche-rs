//! Define what we consider a "container", that is, a file format.
//!
//! This is different from a "data" format which is why it is here.
//!
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum::{EnumString, VariantNames};
use tabled::{builder::Builder, settings::Style};

/// Current `containers.hcl` version (forked from `formats.hcl`).
///
const CVERSION: usize = 2;

/// For each format, we define a set of key attributes that will get displayed.
///
#[derive(Debug, Deserialize)]
pub struct ContainerDescr {
    /// Type of data each format refers to
    #[serde(rename = "type")]
    pub dtype: String,
    /// Free text description
    pub description: String,
    /// Source
    pub source: String,
    /// URL to the site where this is defined
    pub url: String,
}

/// Struct to be read from an HCL file at compile-time
///
#[derive(Debug, Deserialize)]
pub struct ContainerFile {
    /// Version
    pub version: usize,
    /// Ordered list of format metadata
    pub format: BTreeMap<String, ContainerDescr>,
}

/// This struct holds the different container formats that we support.
///
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    strum::Display,
    EnumString,
    Serialize,
    VariantNames,
)]
#[strum(serialize_all = "PascalCase", ascii_case_insensitive)]
pub enum Container {
    /// Common CSV format.
    CSV,
    /// Apache Parquet
    Parquet,
    /// RAW Files
    #[default]
    Raw,
}

impl Container {
    /// List all supported container formats into a string using `tabled`.
    ///
    pub fn list() -> eyre::Result<String> {
        let descr = include_str!("containers.hcl");
        let fstr: ContainerFile = hcl::from_str(descr)?;

        // Safety checks
        //
        assert_eq!(fstr.version, CVERSION);

        let header = vec!["Name", "Type", "Description"];

        let mut builder = Builder::default();
        builder.push_record(header);

        fstr.format.iter().for_each(|(name, entry)| {
            let mut row = vec![];

            let name = name.clone();
            let dtype = entry.dtype.clone();
            let description = entry.description.clone();
            let source = entry.source.clone();
            let url = entry.url.clone();

            let row_text = format!("{}\nSource: {} -- URL: {}", description, source, url);
            let dtype = dtype.to_string();
            row.push(&name);
            row.push(&dtype);
            row.push(&row_text);
            builder.push_record(row);
        });
        let allf = builder.build().with(Style::modern()).to_string();
        let str = format!("List all formats:\n{allf}");
        Ok(str)
    }
}
