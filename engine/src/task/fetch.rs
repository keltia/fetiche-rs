//! `Fetch` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::sync::Arc;
use std::sync::mpsc::Sender;

use anyhow::{anyhow, Result};
use log::trace;

use engine_macros::RunnableDerive;
use fetiche_sources::{Filter, Flow, Site, Sources};

use crate::Runnable;

/// The Fetch task
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Fetch {
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
    pub fn new(s: &str, srcs: Arc<Sources>) -> Self {
        Self {
            name: s.to_string(),
            args: String::new(),
            site: None,
            srcs: Arc::clone(&srcs),
        }
    }
    /// Copy the site's data
    ///
    pub fn site(&mut self, s: String) -> &mut Self {
        trace!("Add site {} as {}", self.name, s);
        self.site = Some(s.to_owned());
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
    fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("Fetch::execute()");
        trace!("received: {}", data);
        // Fetch data as bytes
        //
        let mut data = vec![];
        match &self.site {
            Some(site) => {
                let site = Site::load(site, &self.srcs)?;
                if let Flow::Fetchable(site) = site {
                    let token = site.authenticate()?;
                    site.fetch(&mut data, &token, &self.args)?;
                }
            }
            None => return Err(anyhow!("no site defined")),
        }
        Ok(stdout.send(String::from_utf8(data.to_vec())?)?)
    }
}
