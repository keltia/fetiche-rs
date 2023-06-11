//! `Fetch` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::trace;
use nom::AsBytes;

use engine_macros::RunnableDerive;
use fetiche_formats::Format;
use fetiche_sources::{Fetchable, Filter};

use crate::{Input, Runnable};

/// The Fetch task
///
#[derive(Debug, RunnableDerive)]
pub struct Fetch {
    /// name for the task
    pub name: String,
    /// Input type, File or Network
    pub input: Input,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Fetch {
    /// Initialize our environment
    ///
    pub fn new(name: &str) -> Self {
        trace!("New Fetch {}", name);
        Fetch {
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

    /// Copy the site's data
    ///
    pub fn site(&mut self, s: Box<dyn Fetchable>) -> &mut Self {
        trace!("Add site {:?}", self.name);
        self.input = Input::Network {
            format: s.format(),
            site: s,
        };
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
    fn transform(&mut self, data: String) -> Result<String> {
        trace!("Fetch::transform()");
        trace!("received: {}", data);
        match &self.input {
            // Input::Network is more complicated and rely on the Site
            //
            Input::Network { site, .. } => {
                // Fetch data as bytes
                //
                let mut data = vec![];
                let token = site.authenticate()?;
                site.fetch(&mut data, &token, &self.args)?;

                Ok(String::from_utf8(data.to_vec())?)
            }
            Input::File { path, .. } => {
                Err(anyhow!("fetching not supported, use Read."))
            }
            Input::Stream { .. } => Err(anyhow!("streaming not supported, use Streamable")),
            Input::Nothing => Err(anyhow!("no formats specified")),
        }
    }
}

impl Default for Fetch {
    fn default() -> Self {
        Fetch::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_new() {
        let t = Fetch::new("foo");

        assert_eq!("foo", t.name);
        match t.input {
            Input::Nothing => (),
            _ => panic!("bad type"),
        }
    }

    #[test]
    fn test_fetch_none() {
        let mut t = Fetch::new("foo");
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
