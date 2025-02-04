//! Define what we consider a "container", that is, a file format.
//!
//! This is different from a "data" format which is why it is here.
//!
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum::VariantNames;
use tabled::{builder::Builder, settings::Style};

/// Represents different supported container formats.
///
/// # Variants
///
/// * `CSV` - Represents the common CSV (Comma-Separated Values) format.
/// * `Parquet` - Represents the Apache Parquet format, commonly used for analytical workloads.
/// * `Raw` - Represents raw file formats with no predefined structure (default variant).
///
/// # Features
///
/// This enum derives several traits for convenience:
/// - `Copy` and `Clone` for value copy and cloning.
/// - `Debug` for formatting enumeration into a debug string.
/// - `Default` to provide a default value (`Raw`).
/// - `Deserialize` and `Serialize` to enable (de)serialization support with `serde`.
/// - `PartialEq` for equality comparisons.
/// - `strum::Display` and `VariantNames` for string representation and listing all variant names.
/// - `strum::EnumString` to enable parsing from strings.
///
/// Additionally, the `strum` attributes enhance case insensitivity and control the serialization format.
///
/// # Example
///
/// ```rust
/// use strum::VariantNames;
/// use fetiche_common::Container;
///
/// // Listing all variants
/// println!("{:?}", Container::VARIANTS); // ["CSV", "Parquet", "Raw"]
///
/// // Parsing from a string
/// let container: Container = "csv".parse().unwrap();
/// assert_eq!(container, Container::CSV);
///
/// // Converting to a string
/// let container = Container::Parquet;
/// assert_eq!(container.to_string(), "Parquet");
/// ```
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    strum::Display,
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

impl From<&str> for Container {
    fn from(path: &str) -> Self {
        let extension = path.rsplit('.').next().unwrap_or_default().to_lowercase();
        match extension.as_str() {
            "csv" => Container::CSV,
            "parquet" => Container::Parquet,
            _ => Container::Raw,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_from_str() {
        assert_eq!(Container::from("data.csv"), Container::CSV);
        assert_eq!(Container::from("data.CSV"), Container::CSV);
        assert_eq!(Container::from("data.parquet"), Container::Parquet);
        assert_eq!(Container::from("data.PARQUET"), Container::Parquet);
        assert_eq!(Container::from("data.txt"), Container::Raw);
        assert_eq!(Container::from("data"), Container::Raw);
        assert_eq!(Container::from("data."), Container::Raw);
        assert_eq!(Container::from(""), Container::Raw);
    }
}

// -----

/// Current `containers.hcl` version (forked from `formats.hcl`).
///
const CVERSION: usize = 2;

/// `ContainerDescr` struct provides key attributes for each format, which helps in
/// describing its nature, metadata, and the source from which it originated.
///
/// # Fields
///
/// * `dtype` - Represents the type of data the format refers to. This field is renamed
///   to `type` during (de)serialization for compatibility.
/// * `description` - Provides a free text description of the data format.
/// * `source` - Indicates the origin or specific tool associated with this data format.
/// * `url` - URL pointing to the definition site or further information about the data format.
///
/// This is primarily used to map detailed metadata about different container formats.
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

///
/// `ContainerFile` is a structure designed to be deserialized from an HCL (HashiCorp Configuration Language) file.
///
/// This struct encapsulates metadata for various container formats, organizing the details of how they are structured and defined.
/// It provides compatibility and structure for dealing with metadata in the application, ensuring format consistency.
///
/// # Fields
///
/// * `version` - Specifies the version of the container format file. This ensures proper versioning and compatibility.
/// * `format` - A `BTreeMap` holding the metadata for each container type, keyed by the container's name.
///
/// Each entry in the `format` field references a `ContainerDescr` structure, which provides descriptive details about that specific format.

#[derive(Debug, Deserialize)]
pub struct ContainerFile {
    /// Version
    pub version: usize,
    /// Ordered list of format metadata
    pub format: BTreeMap<String, ContainerDescr>,
}

impl Container {
    ///
    /// List all supported container formats into a string using `tabled`.
    ///
    /// This method loads a predefined HCL file (`containers.hcl`) containing metadata
    /// about different container formats. It ensures the file version matches the expected version
    /// (`CVERSION`) for compatibility. For each container, it generates a modern-styled table with
    /// the container's name, type, description, origin, and associated URL.
    ///
    /// # Returns
    ///
    /// Returns a formatted string representation of the container formats table.
    /// Returns an error if there is an issue in parsing the HCL file or during processing.
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
