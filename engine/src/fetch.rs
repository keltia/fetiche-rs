//! `Fetch` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::sync::Arc;
use anyhow::{anyhow, Result};
use log::trace;

use engine_macros::RunnableDerive;
use fetiche_sources::{Fetchable, Filter};

use crate::Runnable;

/// The Fetch task
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Fetch {
    /// name for the task
    pub name: String,
    /// Site
    pub site: Option<Arc<dyn Fetchable>>,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Fetch {
    pub fn new(s: &str) -> Self {
        Self {
            name: s.to_string(),
            args: String::new(),
            site: None,
        }
    }
    /// Copy the site's data
    ///
    pub fn site(&mut self, s: Arc<dyn Fetchable>) -> &mut Self {
        trace!("Add site {} as {}", self.name, s.name());
        self.site = Some(s);
        self
    }

    /// Add a date filter if specified
    ///
    pub fn with(&mut self, f: Filter) -> &mut Self {
        trace!("Add filter {}", f);
        self.args = f.into();
        self
    }

    /// The heart of the matter: fetch data
    ///
    fn transform(&mut self, data: String) -> Result<String> {
        trace!("Fetch::transform()");
        trace!("received: {}", data);
        // Fetch data as bytes
        //
        let mut data = vec![];
        match &self.site {
            Some(site) => {
                let token = site.authenticate()?;
                site.fetch(&mut data, &token, &self.args)?;
            },
            None => Err(anyhow!("no site defined")),
        }
        Ok(String::from_utf8(data.to_vec())?)
    }
}

