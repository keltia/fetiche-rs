//! `Fetch` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::sync::mpsc::Sender;

use eyre::Result;
use tracing::trace;

use fetiche_macros::RunnableDerive;

use crate::sources::Fetchable;
use crate::{
    EngineStatus, FetchableSource, Filter, Producer, Runnable, Site,
    Stats, IO,
};

/// The Fetch task
///
#[derive(Clone, Debug, PartialEq, RunnableDerive)]
pub struct Fetch {
    /// I/O capabilities
    io: IO,
    /// name for the task
    pub name: String,
    /// Site
    pub site: Option<Site>,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl From<Fetch> for Producer {
    fn from(f: Fetch) -> Self {
        Producer::Fetch(f)
    }
}

impl Fetch {
    #[tracing::instrument]
    pub fn new(s: &str) -> Self {
        Self {
            io: IO::Producer,
            name: s.to_string(),
            args: String::new(),
            site: None,
        }
    }

    /// Copy the site's data
    ///
    #[tracing::instrument(skip(self))]
    pub fn site(&mut self, s: Site) -> &mut Self {
        self.site = Some(s);
        self
    }

    /// Add a date middle if specified
    ///
    #[tracing::instrument(skip(self))]
    pub fn with(&mut self, f: Filter) -> &mut Self {
        self.args = f.to_string();
        self
    }

    /// The heart of the matter: fetch data
    ///
    #[tracing::instrument(skip(self, _data))]
    async fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<Stats> {
        let site = self.site.clone();
        let stats = match site {
            Some(site) => {
                trace!("Site: {}", site.name);

                let src = FetchableSource::from(site);

                // Stream data as bytes
                //
                let token = src.authenticate().await?;

                trace!("execute:args={}", self.args);
                let args = self.args.clone();
                let res = src.fetch(stdout, &token, &args).await?;
                res
            }
            _ => return Err(EngineStatus::NoSiteDefined.into()),
        };
        Ok(stats)
    }
}
