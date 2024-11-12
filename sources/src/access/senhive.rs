//! This is the module for the Thales Senhive antenna
//!

use std::str::FromStr;
use std::sync::mpsc::Sender;

use crate::{Auth, AuthError, Capability, Site, Streamable};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, trace};

use fetiche_formats::Format;

/// AMQP default site
const DEF_AMQP: &str = "senegress.senair.io:5672";
/// Default vhost
const DEF_VHOST: &str = "eurocontrol";

/// Credentials to submit to the site to get the token
///
#[derive(Debug, Deserialize, Serialize)]
struct Credentials {
    /// Email as username
    username: String,
    /// Password
    password: String,
}

#[derive(Clone, Debug, Serialize)]
enum StatMsg {
    Pkts(u32),
    Bytes(u64),
    Reconnect,
    Empty,
    Error,
    Print,
    Exit,
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
    /// Running time (for streams)
    pub duration: i32,
}

impl Senhive {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("senhive::new");
        Senhive {
            features: vec![Capability::Stream],
            format: Format::Senhive,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            vhost: "".to_owned(),
            duration: 0,
        }
    }

    #[tracing::instrument]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        self.format = Format::from_str(&site.format).unwrap();
        self.base_url = site.base_url.to_owned();
        if let Some(auth) = &site.auth {
            match auth {
                Auth::Vhost { vhost, username, password } => {
                    self.vhost = vhost.to_owned();
                    self.login = username.to_owned();
                    self.password = password.to_owned();
                }
                _ => {
                    error!("Bad auth parameter: {}", json!(auth));
                    panic!("nope");
                }
            }
        }
        self
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
        Format::Senhive
    }
}
