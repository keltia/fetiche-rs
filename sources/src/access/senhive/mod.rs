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
//! This is using the new `AsyncStreamable` trait.

mod actors;
mod stream;

use std::io::Cursor;
use std::num::NonZeroUsize;
use std::str::FromStr;

use csv::{QuoteStyle, WriterBuilder};
use eyre::Result;
use lapin::options::BasicConsumeOptions;
use lapin::types::FieldTable;
use lapin::{Connection, Consumer};
use polars::io::{SerReader, SerWriter};
use polars::prelude::{JsonFormat, JsonReader, JsonWriter};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{error, trace};

use crate::{Auth, Capability, Site};
use fetiche_formats::senhive::FusedData;
use fetiche_formats::{DronePoint, Format};

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
        if let Some(auth) = &site.auth {
            match auth {
                Auth::Vhost {
                    vhost,
                    username,
                    password,
                } => {
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
        let base_url = site.base_url.to_owned();
        self.base_url = format!(
            "amqp://{}:{}@{}/{}",
            self.login, self.password, base_url, self.vhost
        );
        self
    }
}

impl Default for Senhive {
    fn default() -> Self {
        Senhive::new()
    }
}

#[derive(Debug)]
pub struct Feed {
    pub name: String,
    pub inp: Consumer,
}

impl Feed {
    pub async fn new(conn: &Connection, name: &str, tag: &str) -> Result<Self> {
        // Create a channel
        let data_ch = conn.create_channel().await?;
        eprintln!("Created {name} channel");

        let data = data_ch
            .basic_consume(
                name,
                tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(Feed {
            name: name.into(),
            inp: data,
        })
    }
}

/// Helper to convert from multi-line JSON into proper JSONL records.
///
#[inline]
fn from_json_to_nl(data: &[u8]) -> Result<String> {
    let cur = Cursor::new(data);
    let mut df = JsonReader::new(cur)
        .with_json_format(JsonFormat::Json)
        .infer_schema_len(NonZeroUsize::new(3))
        .finish()?;

    let mut buf = vec![];
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::JsonLines)
        .finish(&mut df)?;
    Ok(String::from_utf8(buf)?)
}

/// Take the JSON and turn it into our own `DronePoint`.
///
#[inline]
fn from_json_to_csv(data: &[u8]) -> Result<String> {
    let cur = Cursor::new(data);
    let data: FusedData = serde_json::from_reader(cur)?;
    let data: DronePoint = (&data).into();

    let mut wtr = WriterBuilder::new()
        .has_headers(false)
        .quote_style(QuoteStyle::NonNumeric)
        .from_writer(vec![]);

    // Insert data
    //
    wtr.serialize(data)?;

    // Output final csv line
    //
    let data = String::from_utf8(wtr.into_inner()?)?;

    Ok(data)
}
