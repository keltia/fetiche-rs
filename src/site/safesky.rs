//! Safesky specifics
//!
//! Phase:
//! 1. obtain an API key from a mail sent to them
//! 2. use x-api-key: KEY for all submissions to public-api.safesky.app/beacons
//!
//! Format is take from the CSV given as an example
//!

use anyhow::Result;
use log::error;
use reqwest::blocking::Client;

use crate::format::{Cat21, Format};
use crate::site::{Fetchable, Site};

#[derive(Clone, Debug)]
pub struct Safesky {
    pub format: Format,
    pub base_url: String,
    pub get: String,
    pub api_key: String,
    pub client: Client,
}

impl Safesky {
    pub fn new() -> Self {
        Safesky {
            format: Format::None,
            base_url: "".to_owned(),
            api_key: "".to_owned(),
            get: "".to_owned(),
            client: Client::new(),
        }
    }

    pub fn load(&mut self, site: &Site) -> &mut Self {
        match site {
            Site::Key {
                format,
                base_url,
                api_key,
                get,
                ..
            } => {
                self.format = format.as_str().into();
                self.base_url = base_url.to_owned();
                self.api_key = api_key.to_owned();
                self.get = get.to_owned();
            }
            _ => {
                error!("Missing config data for {site:?}")
            }
        }
        self
    }
}

impl Default for Safesky {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Safesky {
    fn authenticate(&self) -> Result<String> {
        todo!()
    }

    fn fetch(&self, _token: &str) -> Result<String> {
        todo!()
    }

    fn process(&self, _input: String) -> Result<Vec<Cat21>> {
        todo!()
    }

    fn format(&self) -> Format {
        Format::Safesky
    }
}
