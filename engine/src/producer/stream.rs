//! `Stream` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fmt::{Debug, Formatter};

use eyre::Result;
use std::sync::mpsc::Sender;
use tracing::trace;

use fetiche_macros::RunnableDerive;

use crate::{
    EngineStatus, Filter, Producer, Runnable, Site,
    Stats, Streamable, StreamableSource, IO,
};

/// The Stream task
///
#[derive(Clone, PartialEq, RunnableDerive)]
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

impl From<Stream> for Producer {
    fn from(f: Stream) -> Self {
        Producer::Stream(f)
    }
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
    #[tracing::instrument(skip(self))]
    pub fn site(&mut self, s: Site) -> &mut Self {
        self.site = Some(s);
        self
    }

    /// Add a date filter if specified
    ///
    #[tracing::instrument(skip(self))]
    pub fn with(&mut self, f: Filter) -> &mut Self {
        self.args = f.to_string();
        self
    }

    /// Set the loop interval
    ///
    #[tracing::instrument(skip(self))]
    pub fn every(&mut self, i: usize) -> &mut Self {
        self.every = i;
        self
    }

    /// The heart of the matter: stream data
    ///
    #[tracing::instrument(skip(self, _data, stdout))]
    pub async fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<Stats> {
        let site = self.site.clone();
        match site {
            Some(site) => {
                trace!("Site: {}", site);

                // Stream data as bytes
                //
                let site = self.site.clone().unwrap();
                match StreamableSource::from(&site) {
                    Some(source) => {
                        let token = source.authenticate()?;

                        let args = self.args.clone();
                        source.stream(stdout, &token, &args)?;
                    }
                    _ => EngineStatus::NotStreamable(site.name.clone()).into(),
                }
            }
            _ => {
                Err(EngineStatus::NoSiteDefined.into())
            }
        };
        Ok(stats)
    }
}
