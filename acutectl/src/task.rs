//! Bare-bone task engine
//!
//! Add a simple API to specify both network and file based input.  Only fetch data, no processing
//! like in `cat21conv`.
//!

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::{debug, trace};

use fetiche_formats::Format;
use fetiche_sources::{Fetchable, Filter};

use crate::Input;

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
        debug!("New task {}", name);
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
        trace!("Add date filter {:?}", f);
        self.args = f.to_string();
        self
    }

    /// The heart of the matter: fetch data
    ///
    pub fn run(&mut self) -> Result<String> {
        trace!("…run()…");
        match &self.input {
            // Input::Network is more complicated and rely on the Site
            //
            Input::Network { site, .. } => {
                // Fetch data as bytes
                //
                let token = site.authenticate()?;
                let data = site.fetch(&token, &self.args)?;
                debug!("{}", &data);
                Ok(data)
            }
            Input::File { path, .. } => Ok(fs::read_to_string(path)?),
            Input::Nothing => Err(anyhow!("no formats specified")),
        }
    }
}

impl Default for Task {
    fn default() -> Self {
        Task::new("default")
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
        match &t.input {
            Input::File { path, format } => {
                assert_eq!(Format::None, *format);
                assert_eq!(PathBuf::from("/nonexistent"), path.clone());
            }
            _ => panic!("bad type"),
        };
    }
}
