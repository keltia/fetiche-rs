//! `Stream` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fmt::{Debug, Formatter};
use std::io::Write;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;

use anyhow::{anyhow, Result};
use log::trace;

use engine_macros::RunnableDerive;
use fetiche_sources::{Filter, Streamable};

use crate::Runnable;

/// The Stream task
///
#[derive(RunnableDerive)]
pub struct Stream {
    /// name for the task
    pub name: String,
    /// Site
    pub site: Option<String>,
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
            site: None,
            args: "".to_string(),
            every: 0,
        }
    }

    /// Copy the site's data
    ///
    pub fn site(&mut self, s: Arc<dyn Streamable>) -> &mut Self {
        trace!("Add site {} as {}", self.name, s.name());
        self.site = Some(s);
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

    /// The heart of the matter: fetch data
    ///
    fn transform(&mut self, data: String) -> Result<String> {
        trace!("Stream::run()");

        // Stream data as bytes
        //
        match &self.site {
            Some(site) => {
                let token = site.authenticate()?;

                let mut data: Vec<u8> = vec![];
                let (tx, rx) = channel::<String>();
                let args = self.args.clone();
                thread::spawn(move || {
                    site.stream(tx.clone(), &token, &self.args)?;
                });

                loop {
                    match rx.recv() {
                        Some(buf) => data.add(&buf),
                        None => break,
                    }
                    return Ok(String::from_utf8(data)?);
                }
            }
            None => return Err(anyhow!("site not defined")),
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
