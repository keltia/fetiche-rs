//! `Fetch` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::sync::mpsc::Sender;

use eyre::Result;
use tracing::trace;

use fetiche_macros::RunnableDerive;

use crate::{AuthError, Capability, EngineStatus, Filter, Runnable, Site, Sources, IO};

/// The Fetch task
///
#[derive(Clone, Debug, RunnableDerive)]
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
    #[tracing::instrument(skip(srcs))]
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

    /// The heart of the matter: fetch data
    ///
    #[tracing::instrument(skip(self))]
    fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("received: {}", data);

        if &self.site.is_none() {
            return Err(EngineStatus::NoSiteDefined.into());
        }
        let site = self.site.clone().expect("Site not defined");
        if site.feature == Capability::Fetch {
            let sources = Sources::new()?;
            let site = sources.as_fetchable(&site.name)?;

            let token = site.authenticate()?;

            // If token has expired
            //
            let token = match token {
                Ok(token) => token,
                Err(e) => match e {
                    AuthError::Expired => site.authenticate()?,
                    _ => return Err(EngineStatus::TokenError(e.to_string()).into()),
                },
            };
            site.fetch(stdout, &token, &self.args)?;
        }
        Ok(())
    }
}
