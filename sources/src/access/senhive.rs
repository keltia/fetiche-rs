//! This is the module for the Thales Senhive antenna
//!

use std::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};

use crate::{AuthError, Capability, Site, Streamable};

use fetiche_formats::Format;

/// Credentials to submit to the site to get the token
///
#[derive(Debug, Deserialize, Serialize)]
struct Credentials {
    /// Email as username
    username: String,
    /// Password
    password: String,
}

#[derive(Clone, Debug)]
pub struct Senhive {
    /// Describe the different features of the source
    pub features: Vec<Capability>,
    /// Input formats
    pub format: Format,
    /// Username
    pub login: String,
    /// Password
    pub password: String,
    /// Base site url taken from config
    pub base_url: String,
    /// Virtual Host
    pub vhost: String,
}

impl Senhive {
    pub fn new() -> Self {
        Senhive {
            features: vec![Capability::Stream],
            format: Format::Senhive,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            vhost: "".to_owned(),
        }
    }

    pub fn load(&self, _s: &Site) -> Self {
        Senhive::default()
    }
}

impl Default for Senhive {
    fn default() -> Self {
        Senhive::new()
    }
}

impl Streamable for Senhive {
    fn name(&self) -> String {
        String::from("Senhive")
    }

    fn authenticate(&self) -> eyre::Result<String, AuthError> {
        todo!()
    }

    fn stream(&self, out: Sender<String>, token: &str, args: &str) -> eyre::Result<()> {
        todo!()
    }

    fn format(&self) -> Format {
        Format::CubeData
    }
}
