//! `Stream` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::{debug, trace};

use fetiche_formats::Format;
use fetiche_sources::{Fetchable, Filter};

use crate::{Input, Runnable};

/// The Stream task
///
#[derive(Debug)]
pub struct Stream {
    /// name for the task
    pub name: String,
    /// Input type, File or Network
    pub input: Input,
    /// Interval in secs
    pub every: usize,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Stream {
    /// Initialize our environment
    ///
    pub fn new(name: &str) -> Self {
        trace!("New Stream {}", name);
        Stream {
            name: name.to_owned(),
            input: Input::Nothing,
            args: "".to_string(),
            every: 0,
        }
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

    /// Set the loop interval
    ///
    pub fn every(&mut self, i: usize) -> &mut Self {
        trace!("Set interval to {} secs", i);
        self.every = i;
        self
    }
}

impl Runnable for Stream {
    /// The heart of the matter: fetch data
    ///
    fn run(&self) -> Result<String> {
        trace!("Stream::run()");
        match &self.input {
            // Input::Network is more complicated and rely on the Site
            //
            Input::Network { site, .. } => {
                // Stream data as bytes
                //
                let token = site.authenticate()?;
                loop {
                    let data = site.fetch(&token, &self.args)?;
                    debug!("{}", &data);
                }
                Ok(data)
            }
            Input::File { path, .. } => Ok(fs::read_to_string(path)?),
            Input::Nothing => Err(anyhow!("no formats specified")),
        }
    }
}

impl Default for Stream {
    fn default() -> Self {
        Stream::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_new() {
        let t = Stream::new("foo");

        assert_eq!("foo", t.name);
        match t.input {
            Input::Nothing => (),
            _ => panic!("bad type"),
        }
    }

    #[test]
    fn test_fetch_none() {
        let mut t = Stream::new("foo");
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
