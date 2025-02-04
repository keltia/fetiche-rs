//! `Fetch` is a `Runnable` task as defined in the `engine`  crate.
//!

use eyre::Result;
use tokio::sync::mpsc::Sender;
use tracing::{error, trace};

use fetiche_macros::RunnableDerive;

use crate::{AuthError, Capability, EngineStatus, FetchableSource, Filter, Runnable, Site, Sources, StreamableSource, IO};

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

    /// Add a date filter if specified
    ///
    pub fn with(&mut self, f: Filter) -> &mut Self {
    #[tracing::instrument(skip(self))]
        self.args = f.to_string();
        self
    }

    /// The heart of the matter: fetch data
    ///
    #[tracing::instrument(skip(self, _data))]
    fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<()> {
        let site = self.site.clone();
        match site {
            Some(site) => {
                trace!("Site: {}", site);

                // Stream data as bytes
                //
                let site = self.site.clone().unwrap();
                match FetchableSource::from(&site) {
                    Some(source) => {
                        let token = source.authenticate()?;

                        let args = self.args.clone();
                        source.stream(stdout, &token, &args)?;
                    }
                    _ => EngineStatus::NotFetchable(site.name.clone()).into(),
                }
            }
            _ => {
                Err(EngineStatus::NoSiteDefined.into())
            }
        }
    }
}
