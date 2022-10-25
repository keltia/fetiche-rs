//! Task-related code
//!

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use csv::ReaderBuilder;
use log::trace;

use crate::site::Fetchable;
use crate::{Cat21, Filter, Source};

#[derive(Debug)]
pub enum Input {
    File {
        format: Source,
        path: PathBuf,
    },
    Network {
        format: Source,
        site: Box<dyn Fetchable + 'static>,
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
        trace!("New task {}", name);
        Task {
            name: name.to_owned(),
            input: Input::Nothing,
            args: Filter::None,
        }
    }

    /// Set the input path (for files)
    ///
    pub fn path(&mut self, name: &str) -> &mut Self {
        trace!("Add path: {}", name);
        let fmt = match &self.input {
            Input::File { format, .. } | Input::Network { format, .. } => format,
            _ => &Source::None,
        };
        self.input = Input::File {
            path: PathBuf::from(name),
            format: fmt.to_owned(),
        };
        self
    }

    /// Set the input format (from cmdline for files)
    ///
    pub fn format(&mut self, fmt: Source) -> &mut Self {
        trace!("Add format {:?}", fmt);
        match &self.input {
            Input::File { path, .. } => {
                let path = path.clone();
                self.input = Input::File { format: fmt, path }
            }
            _ => (),
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
        trace!("Add date filter {:?}", f);
        self.args = f.to_owned();
        self
    }

    /// The heart of the matter: fetch and process data
    ///
    pub fn run(&mut self) -> Result<Vec<Cat21>> {
        trace!("…run()…");
        let (res, format) = match &self.input {
            Input::File { format, path } => {
                let res = fs::read_to_string(path)?;
                (res, format)
            }
            Input::Network { format, site } => {
                // Fetch data as bytes
                //
                let token = site.authenticate()?;
                let res = site.fetch(&token)?;
                (res, format)
            }
            Input::Nothing => return Err(anyhow!("no format specified")),
        };
        let mut rdr = ReaderBuilder::new()
            .flexible(true)
            .from_reader(res.as_bytes());
        format.process(&mut rdr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let t = Task::new("foo");

        assert_eq!("foo", t.name);
        assert_eq!(Input::Nothing, t.input);
    }

    #[test]
    fn test_task_none() {
        let mut t = Task::new("foo");

        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        assert_eq!(
            Input::File {
                format: Source::None,
                path: PathBuf::from("/nonexistent"),
            },
            t.input
        );
    }
}
