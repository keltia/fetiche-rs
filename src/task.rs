//! Task-related code
//!

use std::io::Read;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use csv::{Reader, ReaderBuilder};
use log::trace;
use serde::de::DeserializeOwned;

use crate::format::aeroscope::Aeroscope;
use crate::format::safesky::Safesky;
use crate::{Cat21, Site, Source};

#[derive(Clone, Debug)]
pub enum Input {
    File { format: Source, path: PathBuf },
    Network { format: Source, site: Site },
    Nothing,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: String,
    pub input: Input,
}

impl Task {
    /// Initialize our environment
    ///
    pub fn new(arg: &str) -> Self {
        trace!("New task {}", arg);
        Task {
            name: arg.to_owned(),
            input: Input::Nothing,
        }
    }

    /// Set the input path (for files)
    ///
    pub fn path(&mut self, name: &str) -> &mut Self {
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
        self.input = match &self.input {
            Input::File { path, .. } => {
                let path = path.clone();
                let inp = Input::File { format: fmt, path };
                inp
            }
            _ => self.input.clone(),
        };
        self
    }

    /// Copy the site's data
    ///
    pub fn with(&mut self, s: Site) -> &mut Self {
        self.input = Input::Network {
            format: s.format(),
            site: s,
        };
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
