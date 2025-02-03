//! `Fetch` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::sync::mpsc::Sender;
use std::sync::Arc;

use eyre::Result;
use tracing::trace;

use fetiche_macros::RunnableDerive;
use fetiche_sources::{AuthError, Filter, Flow, Site, Sources};

use crate::{EngineStatus, Runnable, IO};

/// The Fetch task
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Fetch {
    /// I/O capabilities
    io: IO,
    /// name for the task
    pub name: String,
    /// Shared ref to the sources parameters
    pub srcs: Arc<Sources>,
    /// Site
    pub site: Option<String>,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Fetch {
    #[tracing::instrument(skip(srcs))]
    pub fn new(s: &str) -> Self {
        Self {
            io: IO::Producer,
            name: s.to_string(),
            args: String::new(),
            site: None,
            srcs: srcs.clone(),
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

    /// The heart of the matter: fetch data
    ///
    #[tracing::instrument(skip(self))]
    fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("Fetch::execute()");
        trace!("received: {}", data);

        if self.site.is_none() {
            return Err(EngineStatus::NoSiteDefined.into());
        }
        let site = self.site.clone().unwrap();

        let site = self.srcs.load(&site)?;
        if let Flow::Fetchable(site) = site {
            let token = site.authenticate();

            // If token has expired
            //

            let token = match token {
                Err(e) => match e {
                    AuthError::Expired => site.authenticate()?,
                    _ => return Err(EngineStatus::TokenError(e.to_string()).into()),
                },
                Ok(token) => token,
            };
            site.fetch(stdout, &token, &self.args)?;
        }
        Ok(())
    }
}
