//! `Stream` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fmt::{Debug, Formatter, Pointer};
use std::io::Write;
use std::sync::mpsc::Sender;
use std::{fs, io};

use anyhow::{anyhow, Result};
use log::{debug, trace};
use nom::combinator::into;

use fetiche_sources::{Fetchable, Filter};

use crate::{Input, Runnable};

/// The Stream task
///
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

impl Debug for Stream {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("name", &self.name)
            .field("input", &self.input)
            .field("every", &self.every)
            .field("args", &self.args)
            .finish()
    }
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
    fn run(&mut self, out: &mut dyn Write) -> Result<()> {
        trace!("Stream::run()");

        match &self.input {
            // Streaming is only supported for Input::Network
            //
            Input::Stream { site, .. } => {
                // Stream data as bytes
                //
                let token = site.authenticate()?;

                Ok(site.stream(out, &token, &self.args)?)
            }
            _ => Err(anyhow!("Only network support streaming")),
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
}
