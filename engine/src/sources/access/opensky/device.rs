//! This is the streaming implementation for Opensky, direct connection to the antenna.
//!
//! FIXME: this is not using an actor

use crate::{AuthError, Capability, Stats, Streamable};
use fetiche_formats::Format;
use serde::Serialize;
use std::sync::mpsc::Sender;

/// Default port for listening
const DEF_PORT: u16 = 30005;

/// This is the struct holding potential parameters to the API
///
#[derive(Debug, Serialize)]
struct Param {
    /// IP of the receiver antenna
    ///
    pub ip: String,
    /// One or more ICAO24 transponder address
    pub icao24: Option<Vec<String>>,
}


pub struct OpenskyDevice {
    /// Describe the different features of the source
    pub feature: Capability,
    /// TCP stream
    pub stream: Option<std::net::TcpStream>,
}

impl OpenskyDevice {
    pub fn new() -> Self {
        Self {
            feature: Capability::Stream,
            stream: None,
        }
    }

    pub fn connect(&mut self, name: &str) -> &mut Self {
        self.stream = Some(std::net::TcpStream::connect(name).unwrap());
        self
    }
}

impl Streamable for OpenskyDevice {
    #[tracing::instrument(skip(self))]
    fn format(&self) -> Format {
        Format::Opensky
    }

    #[tracing::instrument(skip(self))]
    fn name(&self) -> String {
        String::new("openskydevice")
    }

    /// Fake function.
    ///
    #[tracing::instrument(skip(self))]
    async fn authenticate(&self) -> eyre::Result<String, AuthError> {
        Ok(String::new())
    }

    #[tracing::instrument(skip(self, out, token))]
    async fn stream(&self, out: Sender<String>, token: &str, args: &str) -> eyre::Result<Stats> {
        todo!()
    }
}
