//! `Read` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::trace;

use fetiche_formats::Format;
use fetiche_sources::{Fetchable, Filter};

use crate::{Input, Runnable};

/// The Read task
///
#[derive(Debug)]
pub struct Read {
    /// name for the task
    pub name: String,
    /// Input type, File or Network
    pub input: Input,
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
            input: Input::Nothing,
            args: "".to_string(),
        }
    }

    /// Set the input path (for files)
    ///
    pub fn path(&mut self, name: &str) -> &mut Self {
        trace!("Add path: {}", name);
        let fmt = match &self.input {
            Input::File { format, .. } | Input::Network { format, .. } => format,
            _ => &Format::None,
        };
        self.input = Input::File {
            path: PathBuf::from(name),
            format: fmt.to_owned(),
        };
        self
    }

    /// Set the input formats (from cmdline for files)
    ///
    pub fn format(&mut self, fmt: Format) -> &mut Self {
        trace!("Add formats {:?}", fmt);
        if let Input::File { path, .. } = &self.input {
            let path = path.clone();
            self.input = Input::File { format: fmt, path }
        }
        self
    }

    /// Add a date filter if specified
    ///
    pub fn with(&mut self, f: Filter) -> &mut Self {
        trace!("Add filter {}", f);
        self.args = f.to_string();
        self
    }
}

impl Runnable for Read {
    /// The heart of the matter: fetch data
    ///
    fn run(&mut self, out: &mut dyn Write) -> Result<()> {
        trace!("Read::run()");
        match &self.input {
            // Input::Network is more complicated and rely on the Site
            //
            Input::File { path, .. } => {
                let r = fs::read_to_string(path)?;
                Ok(write!(out, "{}", r)?)
            }
            Input::Network { .. } | Input::Stream { .. } => {
                Err(anyhow!("streaming not supported, use Streamable"))
            }
            Input::Nothing => Err(anyhow!("no formats specified")),
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
        match t.input {
            Input::Nothing => (),
            _ => panic!("bad type"),
        }
    }

    #[test]
    fn test_fetch_none() {
        let mut t = Read::new("foo");
        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        match &t.input {
            Input::File { path, format } => {
                assert_eq!(Format::None, *format);
                assert_eq!(PathBuf::from("/nonexistent"), path.clone());
            }
            _ => panic!("bad type"),
        };
    }
}
