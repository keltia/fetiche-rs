//! Task-related code
//!

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use csv::ReaderBuilder;
use log::trace;

use crate::site::Fetchable;
use crate::{Cat21, Source};

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
pub struct Task {
    /// name for the task
    pub name: String,
    /// Input type, File or Network
    pub input: Input,
    /// Optional arguments
    pub args: Option<String>,
}

impl Task {
    /// Initialize our environment
    ///
    pub fn new(name: &str) -> Self {
        trace!("New task {}", name);
        Task {
            name: name.to_owned(),
            input: Input::Nothing,
            args: None,
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
        self.input = match &self.input {
            Input::File { path, .. } => {
                let path = path.clone();
                Input::File { format: fmt, path }
            }
            _ => self.input.clone(),
        };
        self
    }

    /// Copy the site's data
    ///
    pub fn site(&mut self, s: Box<dyn Fetchable>) -> &mut Self {
        trace!("Add site {:?}", s);
        self.input = Input::Network {
            format: s.format(),
            site: s,
        };
        self
    }

    /// Copy arguments if needed
    ///
    pub fn with(&mut self, arg: &str) -> &mut Self {
        self.args = Some(arg.to_owned());
        self
    }

    /// The heart of the matter: fetch and process data
    ///
    pub fn run(&mut self) -> Result<Vec<Cat21>> {
        trace!("…run()…");
        match &self.input {
            Input::File { format, path } => {
                let mut rdr = ReaderBuilder::new().flexible(true).from_path(path)?;
                format.process(&mut rdr)
            }
            Input::Network { format, site } => {
                // Fetch data as bytes
                //
                let res = site.fetch()?;

                let mut rdr = ReaderBuilder::new()
                    .flexible(true)
                    .from_reader(res.as_bytes());
                format.process(&mut rdr)
            }
            Input::Nothing => Err(anyhow!("no format specified")),
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
