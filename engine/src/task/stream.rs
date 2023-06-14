//! `Stream` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use anyhow::{anyhow, Result};
use log::trace;

use engine_macros::RunnableDerive;
use fetiche_sources::{Filter, Flow, Site, Sources};

use crate::Runnable;

/// The Stream task
///
#[derive(Clone, RunnableDerive)]
pub struct Stream {
    /// name for the task
    pub name: String,
    /// Shared ref to configuration
    pub srcs: Arc<Sources>,
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
            .field("site", &self.site)
            .field("srcs", &self.srcs)
            .field("every", &self.every)
            .field("args", &self.args)
            .finish()
    }
}

impl Stream {
    /// Initialize our environment
    ///
    pub fn new(name: &str, srcs: Arc<Sources>) -> Self {
        trace!("New Stream {}", name);
        Stream {
            name: name.to_owned(),
            site: None,
            srcs: Arc::clone(&srcs),
            args: "".to_string(),
            every: 0,
        }
    }

    /// Copy the site's data
    ///
    pub fn site(&mut self, s: String) -> &mut Self {
        trace!("Add site {} as {}", self.name, s);
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
    pub fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<()> {
        trace!("Stream::run()");

        // Stream data as bytes
        //
        match &self.site {
            Some(site) => {
                let site = Site::load(site, &self.srcs)?;
                if let Flow::Streamable(site) = site {
                    let token = site.authenticate()?;

                    let args = self.args.clone();
                    site.stream(stdout, &token, &args).unwrap();
                }
            },
            None => return Err(anyhow!("site not defined")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
