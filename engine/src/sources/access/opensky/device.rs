//! This is the streaming implementation for Opensky, direct connection to the antenna.
//!
//! There are different ports available, each with a specific output format:
//!
//! - 30001: RAW TCP
//! - 30002: Raw MODE-S frames
//! - 30003: CSV
//! - 30004:
//! - 30005: Binary format
//!
//! FIXME: this is not using an actor

use std::sync::mpsc::Sender;

use eyre::Result;
use nom::Parser;
use ractor::ActorRef;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

use crate::actors::StatsMsg;
use crate::{AuthError, Capability, ParamError, Site, Stats, Streamable};
use fetiche_formats::Format;

/// Default port for data, we prefer the straight CSV output
const DEF_PORT: u16 = 30003;

/// This is the struct holding potential parameters to the API
///
#[derive(Debug, Deserialize, Serialize)]
struct Param {
    /// IP of the receiver antenna
    ///
    pub ip: String,
    /// One or more ICAO24 transponder address
    pub icao24: Option<Vec<String>>,
}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OpenskyDevice {
    /// TCP stream
    stream: Option<String>,
    /// Stats actor ref
    stats: Option<ActorRef<StatsMsg>>,
}

impl OpenskyDevice {
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            stream: None,
            stats: None,
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, _site: Site) -> &mut Self { self }

    #[tracing::instrument(skip(self))]
    pub fn connect(&mut self, name: &str) -> &mut Self {
        self.stream = Some(name.to_string());
        self
    }

    #[tracing::instrument(skip(self))]
    pub fn stats(&mut self, stats: ActorRef<StatsMsg>) -> &mut Self {
        self.stats = Some(stats);
        self
    }

    #[tracing::instrument(skip(self))]
    pub fn feature(&self) -> Result<Capability> {
        Ok(Capability::Stream)
    }
}

impl Streamable for OpenskyDevice {
    #[tracing::instrument(skip(self))]
    fn name(&self) -> String {
        String::from("openskydevice")
    }

    /// Fake function.
    ///
    #[tracing::instrument(skip(self))]
    async fn authenticate(&self) -> eyre::Result<String, AuthError> {
        Ok(String::new())
    }

    #[tracing::instrument(skip(self, out, _token))]
    async fn stream(&self, out: Sender<String>, _token: &str, _args: &str) -> Result<Stats> {
        //let args: Param = serde_json::from_str(args)?;

        let tag = self.name().clone();
        let stat = self.stats.clone().ok_or(Err(ParamError::NoStatsActor).into())?;

        stat.cast(StatsMsg::Reset(tag.clone()))?;

        // Get addr from parameters and connect
        //
        let addr = self.stream.clone().ok_or(Err(ParamError::NoAddrGiven).into())?;
        let flow = TcpStream::connect(addr).await?;
        let mut flow = BufReader::new(flow).lines();
        let mut st = Stats::default();

        // Read every line until… whatever
        //
        while let Some(line) = flow.next_line().await? {
            let len = line.len();
            st.pkts += 1;
            st.bytes = st.bytes + len as u64;
            let _ = out.send(line.clone())?;
        };

        stat.cast(StatsMsg::Update(tag, Stats {
            bytes: st.bytes,
            pkts: st.pkts,
            ..Default::default()
        }))?;

        Ok(st)
    }

    #[tracing::instrument(skip(self))]
    fn format(&self) -> Format {
        Format::Opensky
    }
}
