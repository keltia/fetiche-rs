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
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::format::Source;
use crate::site::{Fetchable, Site};

const NAME: &str = "safesky";

#[derive(Clone, Debug)]
pub struct Safesky {
    pub format: Source,
    pub base_url: String,
    pub get: String,
    pub api_key: String,
}

impl Safesky {
    pub fn new() -> Self {
        Safesky {
            format: Source::None,
            base_url: "".to_owned(),
            api_key: "".to_owned(),
            get: "".to_owned(),
        }
    }

    pub fn load(&mut self, cfg: &Config) -> &mut Self {
        match &cfg.sites[NAME] {
            Site::Key {
                format,
                base_url,
                api_key,
                get,
                ..
            } => {
                self.format = Source::from_str(format);
                self.base_url = base_url.to_owned();
                self.api_key = api_key.to_owned();
                self.get = get.to_owned();
            }
            _ => {
                error!("Missing config data for {NAME}")
            }
        }
        self
    }
}

impl Fetchable for Safesky {
    fn authenticate(&self) -> Result<String> {
        todo!()
    }

    fn fetch(&self, _token: &str) -> Result<String> {
        todo!()
    }

    fn prefetch(&self, _token: &str) -> Result<String> {
        todo!()
    }

    fn format(&self) -> Source {
        Source::Safesky
    }
}
