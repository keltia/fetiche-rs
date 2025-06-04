//! This is the module for the Thales Senhive antenna
//!
//! The Senhive system uses AMQP to send out different kind of messages:
//!
//! - regular data from the `fused_data`  topic.  Each message has to be ACK'd within 5s
//! - system alerts from the `system_alert` topic
//! - system state from the `system_state`.  This is sent every minute.
//!
//! If any of these messages are not ACK'd within 5s, they are move to the Dead Letter equivalent queues:
//! - `dl_fused_data`
//! - `dl_system_alert`
//! - `dl_ system_state`
//!
//! System state messages are not that interesting but serve as a kind of watchdog.
//!
//! So our principle is, in order to never lose a message, is to start by draining the `dl_fused_data` topic,
//! then switch to the regular `fused_data` topic.
//!
//! This is using the new `Streamable` trait.

mod actors;
mod feed;
mod stream;
mod subr;

pub(crate) use feed::*;
pub(crate) use subr::*;

use std::str::FromStr;

use crate::actors::StatsMsg;
use crate::{Auth, Capability, Site, StreamableSource};
use fetiche_formats::Format;
use polars::io::{SerReader, SerWriter};
use ractor::ActorRef;
use serde::{Deserialize, Serialize};

/// Senhive process/actor group
pub(crate) const SENHIVE_PG: &str = "senhive-pg";

/// Credentials to submit to the site to get the token
///
#[derive(Debug, Deserialize, Serialize)]
struct Credentials {
    /// Email as username
    username: String,
    /// Password
    password: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Senhive {
    /// Describe the different features of the source
    pub feature: Capability,
    /// Input formats
    pub format: Format,
    /// Base site url taken from config
    pub base_url: String,
    /// Running time (for streams)
    pub duration: i32,
    /// Stats gathering actor
    #[serde(skip)]
    pub stat: Option<ActorRef<StatsMsg>>,
}

impl Senhive {
    #[tracing::instrument]
    pub fn new() -> Self {
        Senhive {
            feature: Capability::Stream,
            format: Format::Senhive,
            base_url: "".to_owned(),
            duration: 0,
            stat: None,
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        self.format = Format::from_str(&site.format).unwrap_or(Format::Senhive);
        if let Some(auth) = &site.auth {
            match auth {
                Auth::Vhost {
                    vhost,
                    username,
                    password,
                } => {
                    self.base_url = format!(
                        "amqp://{username}:{password}@{}/{vhost}",
                        site.base_url
                    );
                }
                _ => {
                    self.base_url = String::new()
                }
            }
        }
        self
    }

    #[tracing::instrument(skip(self, stat))]
    pub fn stats(&mut self, stat: ActorRef<StatsMsg>) -> &mut Self {
        self.stat = Some(stat);
        self
    }

    #[tracing::instrument(skip(self))]
    pub fn source(&self) -> StreamableSource {
        StreamableSource::Senhive(self.clone())
    }
}

impl Default for Senhive {
    fn default() -> Self {
        Senhive::new()
    }
}

