//! Streams line-oriented data from an OpenSky receiver over TCP.

use std::net::Ipv6Addr;

use eyre::Result;
use ractor::ActorRef;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;

use crate::actors::StatsMsg;
use crate::{AccessError, AuthError, Capability, ParamError, Site, Stats, Streamable};
use fetiche_formats::Format;

const DEF_PORT: u16 = 30003;

/// A local OpenSky receiver with an optional statistics observer.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct OpenskyDevice {
    stream: Option<String>,
    #[serde(skip)]
    stats: Option<ActorRef<StatsMsg>>,
}

impl OpenskyDevice {
    /// Creates a device with no configured address.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the receiver address from the source's base URL.
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        self.stream = (!site.base_url.is_empty()).then(|| site.base_url.clone());
        self
    }

    /// Sets a host, host and port, or `tcp://` endpoint for direct streaming.
    #[tracing::instrument(skip(self, name))]
    pub fn connect(&mut self, name: &str) -> &mut Self {
        self.stream = Some(name.to_owned());
        self
    }

    /// Registers an optional observer for final stream statistics.
    #[tracing::instrument(skip(self, stats))]
    pub fn stats(&mut self, stats: ActorRef<StatsMsg>) -> &mut Self {
        self.stats = Some(stats);
        self
    }

    /// Reports that this source supports streaming.
    #[tracing::instrument(skip(self))]
    pub fn feature(&self) -> Result<Capability> {
        Ok(Capability::Stream)
    }
}

impl Streamable for OpenskyDevice {
    /// Returns the adapter identifier.
    fn name(&self) -> String {
        "openskydevice".into()
    }

    /// Direct receivers use no authentication token.
    async fn authenticate(&self) -> Result<String, AuthError> {
        Ok(String::new())
    }

    /// Streams text lines until the receiver closes, awaiting downstream capacity.
    #[tracing::instrument(skip_all)]
    async fn stream(&self, out: Sender<String>, _token: &str, _args: &str) -> Result<Stats> {
        let address = self.stream.as_deref().ok_or(ParamError::NoAddrGiven)?;
        let (host, port) = receiver_endpoint(address)?;
        let stream = TcpStream::connect((host.as_str(), port)).await?;
        let stats = forward_lines(BufReader::new(stream), out).await?;
        if let Some(observer) = &self.stats {
            observer.cast(StatsMsg::Update(self.name(), stats.clone()))?;
        }
        Ok(stats)
    }

    /// Returns the OpenSky data format.
    fn format(&self) -> Format {
        Format::Opensky
    }
}

/// Parses the receiver address and applies the default CSV port when omitted.
fn receiver_endpoint(address: &str) -> Result<(String, u16)> {
    let invalid = || AccessError::BadParam("invalid OpenSky TCP endpoint".into());
    if let Ok(address) = address.parse::<Ipv6Addr>() {
        return Ok((address.to_string(), DEF_PORT));
    }
    let url = if address.contains("://") {
        Url::parse(address)
    } else {
        Url::parse(&format!("tcp://{address}"))
    }
    .map_err(|_| invalid())?;
    if url.scheme() != "tcp" || !url.username().is_empty() || url.password().is_some() {
        return Err(invalid().into());
    }
    let host = url.host_str().ok_or_else(invalid)?;
    Ok((
        host.trim_matches(['[', ']']).to_owned(),
        url.port().unwrap_or(DEF_PORT),
    ))
}

/// Sends each received line and counts the bytes actually forwarded.
async fn forward_lines(input: impl AsyncBufRead + Unpin, out: Sender<String>) -> Result<Stats> {
    let mut lines = input.lines();
    let mut stats = Stats::default();
    while let Some(line) = lines.next_line().await? {
        let bytes = line.len() as u64;
        out.send(line).await?;
        stats.pkts += 1;
        stats.bytes += bytes;
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source configuration supplies the receiver endpoint and keeps runtime handles out of serde.
    #[test]
    fn receiver_uses_configured_host_and_port() -> Result<()> {
        let site = Site {
            base_url: "tcp://127.0.0.1:30005".into(),
            ..Site::default()
        };
        let mut device = OpenskyDevice::new();
        device.load(&site);
        assert_eq!(
            receiver_endpoint(device.stream.as_deref().unwrap())?,
            ("127.0.0.1".into(), 30005)
        );
        assert_eq!(
            receiver_endpoint("localhost")?,
            ("localhost".into(), DEF_PORT)
        );
        assert!(receiver_endpoint("https://localhost").is_err());
        let restored: OpenskyDevice = serde_json::from_str(&serde_json::to_string(&device)?)?;
        assert_eq!(restored.stream, device.stream);
        assert!(restored.stats.is_none());
        Ok(())
    }

    /// The direct adapter forwards lines in order and counts their payload lengths.
    #[tokio::test]
    async fn receiver_forwards_lines_and_counts_bytes() -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        let stats = forward_lines(BufReader::new(&b"a,b\nc,d\n"[..]), tx).await?;
        assert_eq!((stats.pkts, stats.bytes), (2, 6));
        assert_eq!(rx.recv().await.as_deref(), Some("a,b"));
        assert_eq!(rx.recv().await.as_deref(), Some("c,d"));
        assert!(rx.recv().await.is_none());
        Ok(())
    }
}
