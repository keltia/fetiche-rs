//! `Read` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use anyhow::{anyhow, Result};
use log::trace;

use engine_macros::RunnableDerive;
use fetiche_formats::Format;
use fetiche_sources::Filter;

use crate::Runnable;

/// The Read task
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Read {
    /// name for the task
    pub name: String,
    /// Format
    pub format: Format,
    /// File path
    pub path: Option<PathBuf>,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Read {
    /// Initialize our environment
    ///
    pub fn new(name: &str) -> Self {
        trace!("New Read {}", name);
        Read {
            name: name.to_owned(),
            format: Format::None,
            path: None,
            args: "".to_string(),
        }
    }

    /// Set the input path (for files)
    ///
    pub fn path(&mut self, name: &str) -> &mut Self {
        trace!("Add path: {}", name);
        self.path = Some(PathBuf::from(name));
        self
    }

    /// Set the input formats (from cmdline for files)
    ///
    pub fn format(&mut self, fmt: Format) -> &mut Self {
        trace!("Add formats {:?}", fmt);
        self.format = fmt;
        self
    }

    /// Add a date filter if specified
    ///
    pub fn with(&mut self, f: Filter) -> &mut Self {
        trace!("Add filter {}", f);
        self.args = f.to_string();
        self
    }

    /// The heart of the matter: fetch data
    ///
    pub fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<()> {
        trace!("Read::transform()");
        if self.path.is_none() || self.format == Format::None {
            Err(anyhow!("uninitialised read"))
        } else {
            let p = self.path.clone().unwrap();
            let r = fs::read_to_string(p)?;
            Ok(stdout.send(r)?)
        }
    }
}

impl Default for Read {
    fn default() -> Self {
        Read::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_new() {
        let t = Read::new("foo");

        assert_eq!("foo", t.name);
        assert!(t.path.is_none());
        assert_eq!(Format::None, t.format);
    }

    #[test]
    fn test_fetch_none() {
        let mut t = Read::new("foo");
        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        assert_eq!(Format::None, t.format);
        assert_eq!(PathBuf::from("/nonexistent"), path.clone());
    }

    #[test]
    fn test_fetch_file() {
        let mut t = Read::new("foo");
        t.path("../Cargo.toml");
        t.format(Format::Asd);

        assert_eq!("foo", t.name);
        assert_eq!(Format::Asd, t.format);
        assert_eq!(PathBuf::from("../Cargo.toml"), path.clone());
    }
}
