//! Task-related code
//!

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use csv::ReaderBuilder;
use log::debug;

use crate::filter::Filter;
use crate::format::{Cat21, Format};
use crate::site::Fetchable;

#[derive(Debug)]
pub enum Input {
    File {
        format: Format,
        path: PathBuf,
    },
    Network {
        format: Format,
        site: Box<dyn Fetchable>,
    },
    Nothing,
}

#[derive(Debug)]
pub struct Task {
    /// name for the task
    pub name: String,
    /// Input type, File or Network
    pub input: Input,
    /// Optional arguments
    pub args: Filter,
}

impl Task {
    /// Initialize our environment
    ///
    pub fn new(name: &str) -> Self {
        debug!("New task {}", name);
        Task {
            name: name.to_owned(),
            input: Input::Nothing,
            args: Filter::None,
        }
    }

    /// Set the input path (for files)
    ///
    pub fn path(&mut self, name: &str) -> &mut Self {
        debug!("Add path: {}", name);
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

    /// Set the input format (from cmdline for files)
    ///
    pub fn format(&mut self, fmt: Format) -> &mut Self {
        debug!("Add format {:?}", fmt);
        if let Input::File { path, .. } = &self.input {
            let path = path.clone();
            self.input = Input::File { format: fmt, path }
        }
        self
    }

    /// Copy the site's data
    ///
    pub fn site(&mut self, s: Box<dyn Fetchable>) -> &mut Self {
        debug!("Add site {:?}", self.name);
        self.input = Input::Network {
            format: s.format(),
            site: s,
        };
        self
    }

    /// Add a date filter if specified
    ///
    pub fn with(&mut self, f: Filter) -> &mut Self {
        debug!("Add date filter {:?}", f);
        self.args = f;
        self
    }

    /// The heart of the matter: fetch and process data
    ///
    pub fn run(&mut self) -> Result<Vec<Cat21>> {
        debug!("…run()…");
        match &self.input {
            // Input::File is simple, we have the format
            //
            Input::File { format, path } => {
                let res = fs::read_to_string(path)?;
                let mut rdr = ReaderBuilder::new()
                    .flexible(true)
                    .from_reader(res.as_bytes());
                format.process(&mut rdr)
            }
            // Input::Network is more complicated and rely on the Site
            //
            Input::Network { format, site } => {
                // Fetch data as bytes
                //
                let token = site.authenticate()?;
                let data = site.fetch(&token)?;
                let res = site.process(data)?;
                debug!("{:?} as {}", res, format);
                Ok(res)
            }
            Input::Nothing => return Err(anyhow!("no format specified")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let t = Task::new("foo");

        assert_eq!("foo", t.name);
        match t.input {
            Input::Nothing => (),
            _ => panic!("bad type"),
        }
    }

    #[test]
    fn test_task_none() {
        let mut t = Task::new("foo");

        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        match t.input {
            Input::File { path, format } => {
                assert_eq!(Format::None, format);
                assert_eq!(PathBuf::from("/nonexistent"), path);
            }
            _ => panic!("bad type"),
        };
    }
}
