//! `Stream` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fmt::{Debug, Formatter};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use eyre::Result;
use tracing::trace;

use fetiche_macros::RunnableDerive;

use crate::{AuthError, Capability, EngineStatus, Filter, Flow, Runnable, Site, IO};

/// The Stream task
///
#[derive(Clone, RunnableDerive)]
pub struct Stream {
    /// I/O capabilities
    io: IO,
    /// name for the task
    pub name: String,
    /// Site
    pub site: Option<Site>,
    /// Interval in secs
    pub every: usize,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Debug for Stream {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("io", &self.io)
            .field("name", &self.name)
            .field("site", &self.site)
            .field("every", &self.every)
            .field("args", &self.args)
            .finish()
    }
}

impl Stream {
    /// Initialize our environment
    ///
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        trace!("New Stream {}", name);
        Stream {
            io: IO::Producer,
            name: name.to_owned(),
            site: None,
            args: "".to_string(),
            every: 0,
        }
    }

    /// Copy the site's data
    ///
    pub fn site(&mut self, s: Site) -> &mut Self {
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
    #[tracing::instrument(skip(self, _data, stdout))]
    pub fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<()> {
        trace!("Stream::run()");

        if self.site.is_none() {
            return Err(EngineStatus::NoSiteDefined.into());
        }
        let site = self.site.clone().unwrap();

        // Stream data as bytes
        //
        let site = self.site.clone().expect("Site not defined");
        if site.feature == Capability::Stream {
            let token = site.authenticate()?;

            let args = self.args.clone();
            site.stream(stdout, &token, &args)?;
        } else if let Flow::AsyncStreamable(site) = site {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async move {
                let token = site.authenticate().await.unwrap();

                let args = self.args.clone();
                site.stream(stdout, &token, &args).await.unwrap();
            })
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
