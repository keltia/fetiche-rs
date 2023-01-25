use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::trace;
use serde::de;

use format_specs::opensky::Opensky;
use format_specs::{Cat21, Format};
use sites::filter::Filter;
use sites::Fetchable;

/// Type of task we will need to do
///
#[derive(Debug)]
pub enum Input {
    /// File-based means we need the format beforehand and a pathname
    ///
    File {
        /// Input format-specs
        format: Format,
        /// Path of the input file
        path: PathBuf,
    },
    /// Network-based means we need the site name (whose details are taken from the configuration
    /// file.  The `site` is a `Fetchable` object generated from `Config`.
    ///
    Network {
        /// Input format-specs
        format: Format,
        /// Site itself
        site: Box<dyn Fetchable>,
    },
    Nothing,
}

/// The task itself
#[derive(Debug)]
pub struct Task {
    /// name for the task
    pub name: String,
    /// Input type, File or Network
    pub input: Input,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Task {
    /// Initialize our environment
    ///
    pub fn new(name: &str) -> Self {
        trace!("New task {}", name);
        Task {
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

    /// Set the input format-specs (from cmdline for files)
    ///
    pub fn format(&mut self, fmt: Format) -> &mut Self {
        trace!("Add format-specs {:?}", fmt);
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
        trace!("Add date filter {:?}", f);
        self.args = f.to_string();
        self
    }

    /// The heart of the matter: fetch and process data
    ///
    pub fn run<T>(&mut self) -> Result<Vec<T>>
    where
        T: de::DeserializeOwned,
    {
        trace!("…run()…");
        let res = match &self.input {
            // Input::File is simple, we have the format
            //
            Input::File { format, path } => {
                let res = fs::read_to_string(path)?;
                let res: Vec<T> = serde_json::from_str(&res)?;
                res
            }
            // Input::Network is more complicated and rely on the Site
            //
            Input::Network { format, site } => {
                // Fetch data as bytes
                //
                let token = site.authenticate()?;
                let data = site.fetch(&token, &self.args)?;
                trace!("{}", &data);
                let res: Vec<T> = serde_json::from_str(&data)?;
                res
            }
            Input::Nothing => return Err(anyhow!("no format specified")),
        };
        Ok(res)
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
