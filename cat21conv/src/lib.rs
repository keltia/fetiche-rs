//! Library part of the Cat21 converter
//!
//! This library include the code for the different file formats used as input and the different
//! way of fetching data from different sources.  This is written because there are as many ways
//! to authenticate and connect as there are sources more or less.
//!
//! The different formats are in the `format-specs` crate and the sources' parameters in the `site` crate.
//!
//! Include Task-related code.
//!
//! A task is a job that we have to perform.  It can be either a file-based or a network-based one.
//! We have a set of methods to add parameter and configure the task then we need to call `run()`
//! to execute it.
//!

use clap::{crate_name, crate_version};

pub(crate) const VERSION: &str = crate_version!();
pub(crate) const NAME: &str = crate_name!();

/// Returns the library version
///
pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
}

/// File-based example:
/// ```no_run
/// # use anyhow::Result;
/// # use std::path::PathBuf;
/// # use log::info;
/// # fn main() -> Result<()> {
/// use cat21conv::Task;
/// use format_specs::{Cat21, Format};
///
/// let what = "foo.json";
/// let format = Format::None;
///
/// let res: Vec<Cat21> = Task::new("foo").path(what).format(format).run()?;
///
/// # Ok(())
/// # }
/// ```
///
/// Network-based example:
/// ```no_run
/// # use anyhow::Result;
/// # use std::path::PathBuf;
/// use cat21conv::Task;
///
/// // Fetch from network
/// //
/// use format_specs::Cat21;
///
/// use sources::Sites;
/// use sources::Filter;
/// use sources::Site;
///
/// # fn main() -> Result<()> {
/// # let name = "eih";
/// # let filter = Filter::None;
///
/// let cfg = Sites::load(&Some(PathBuf::from("config.hcl")))?;
///
/// let site = Site::load(name, &cfg)?;
/// let res: Vec<Cat21> = Task::new(name).site(site).with(filter).run()?;
///
/// # Ok(())
/// # }
/// ```
///
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use csv::ReaderBuilder;
use log::debug;

use format_specs::{Cat21, Format};

use sources::{Fetchable, Filter};

/// Type of task we will need to do
///
#[derive(Debug)]
pub enum Input {
    /// File-based means we need the format-specs beforehand and a pathname
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

    /// Set the input format-specs (from cmdline for files)
    ///
    pub fn format(&mut self, fmt: Format) -> &mut Self {
        debug!("Add format-specs {:?}", fmt);
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
        self.args = f.to_string();
        self
    }

    /// The heart of the matter: fetch and process data
    ///
    pub fn run(&mut self) -> Result<Vec<Cat21>> {
        debug!("…run()…");
        match &self.input {
            // Input::File is simple, we have the format-specs
            //
            Input::File { format, path } => {
                let res = fs::read_to_string(path)?;
                let mut rdr = ReaderBuilder::new()
                    .flexible(true)
                    .from_reader(res.as_bytes());
                format.from_csv(&mut rdr)
            }
            // Input::Network is more complicated and rely on the Site
            //
            Input::Network { format, site } => {
                // Fetch data as bytes
                //
                let token = site.authenticate()?;
                let data = site.fetch(&token, &self.args)?;
                debug!("{}", &data);
                let res = site.process(data)?;
                debug!("{:?} as {}", res, format);
                Ok(res)
            }
            Input::Nothing => Err(anyhow!("no format-specs specified")),
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
