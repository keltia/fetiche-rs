//! Polls an OpenSky HTTP endpoint and streams new snapshots downstream.

use std::fmt;
use std::time::Duration;

use eyre::{Result, eyre};
use jiff::Timestamp;
use mini_moka::sync::Cache;
use ractor::ActorRef;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize, de::IgnoredAny};
use tokio::sync::mpsc::Sender;
use tokio::time::{Instant, sleep, timeout};
use tracing::debug;

use super::{CACHE_IDLE, CACHE_MAX, CACHE_SIZE, MAX_INTERVAL};
use crate::actors::StatsMsg;
use crate::{
    AccessError, Auth, AuthError, Capability, Filter, Site, Stats, Streamable, StreamableSource,
};
use fetiche_formats::Format;

/// An HTTP polling adapter for OpenSky-compatible state endpoints.
#[derive(Clone, Deserialize, Serialize)]
pub struct OpenskyServer {
    pub feature: Capability,
    pub format: Format,
    pub base_url: String,
    pub get: String,
    pub duration: i32,
    #[serde(skip)]
    pub stats: Option<ActorRef<StatsMsg>>,
    #[serde(skip)]
    auth: Option<Auth>,
}

impl fmt::Debug for OpenskyServer {
    /// Reports adapter metadata without exposing the endpoint or credentials.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenskyServer")
            .field("feature", &self.feature)
            .field("format", &self.format)
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}

impl OpenskyServer {
    /// Creates an unconfigured HTTP polling adapter.
    pub fn new() -> Self {
        Self {
            feature: Capability::Stream,
            format: Format::Opensky,
            base_url: String::new(),
            get: String::new(),
            duration: 0,
            stats: None,
            auth: None,
        }
    }

    /// Loads the base URL, stream route, and optional HTTP Basic credentials.
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        self.base_url.clone_from(&site.base_url);
        self.get = site.route("stream").cloned().unwrap_or_default();
        self.auth = site.auth.clone();
        self
    }

    /// Registers an optional observer for final polling statistics.
    pub fn stats(&mut self, stats: ActorRef<StatsMsg>) -> &mut Self {
        self.stats = Some(stats);
        self
    }

    /// Wraps this polling adapter in the engine's dispatch enum.
    pub fn source(&self) -> StreamableSource {
        StreamableSource::OpenskyServer(self.clone())
    }

    /// Appends the configured route and encodes a query parameter.
    fn endpoint(&self, query: Option<(&str, &str)>) -> Result<Url> {
        let invalid = || AccessError::BadParam("invalid OpenSky HTTP endpoint".into());
        let mut url = Url::parse(&self.base_url).map_err(|_| invalid())?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(invalid().into());
        }
        if !self.get.is_empty() {
            let path = format!(
                "{}/{}",
                url.path().trim_end_matches('/'),
                self.get.trim_start_matches('/')
            );
            url.set_path(&path);
        }
        if let Some((name, value)) = query {
            url.query_pairs_mut().append_pair(name, value);
        }
        Ok(url)
    }

    /// Requests snapshots serially so backpressure limits how quickly the adapter polls.
    async fn poll(
        &self,
        url: &Url,
        delay: Duration,
        out: &Sender<String>,
        stats: &mut Stats,
    ) -> Result<()> {
        let client = Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(reqwest::Error::without_url)?;
        let cache = Cache::builder()
            .max_capacity(CACHE_SIZE)
            .time_to_idle(CACHE_IDLE)
            .time_to_live(CACHE_MAX)
            .build();
        loop {
            let mut request = client.get(url.clone());
            if let Some(Auth::Login { username, password }) = &self.auth {
                request = request.basic_auth(username, Some(password));
            }
            match request.send().await {
                Ok(response) if response.status() == StatusCode::OK => {
                    let text = response.text().await.map_err(reqwest::Error::without_url)?;
                    forward_snapshot(text, &cache, out, stats).await?;
                }
                Ok(response) => {
                    let status = response.status();
                    stats.err += 1;
                    if status.is_client_error() && status != StatusCode::TOO_MANY_REQUESTS {
                        return Err(eyre!("OpenSky request failed with HTTP {status}"));
                    }
                    debug!(%status, "Retrying OpenSky request");
                }
                Err(error) => {
                    stats.err += 1;
                    debug!(error = %error.without_url(), "Retrying OpenSky request");
                }
            }
            sleep(delay).await;
        }
    }
}

impl Default for OpenskyServer {
    /// Returns an unconfigured HTTP polling adapter.
    fn default() -> Self {
        Self::new()
    }
}

impl Streamable for OpenskyServer {
    /// Returns the adapter identifier.
    fn name(&self) -> String {
        "openskyserver".into()
    }

    /// Uses the site's credentials for each request. No separate token is needed.
    async fn authenticate(&self) -> Result<String, AuthError> {
        Ok(String::new())
    }

    /// Polls until the configured duration elapses or an error occurs.
    /// Zero duration continues until cancellation or downstream closure.
    #[tracing::instrument(skip_all)]
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<Stats> {
        let now = Timestamp::now().as_second();
        let (seconds, delay, query) = match Filter::from(args) {
            Filter::Stream {
                from,
                duration,
                delay,
            } => (
                u64::from(duration),
                Duration::from_millis(u64::from(delay.max(1))),
                (from != 0).then(|| {
                    (
                        "time".to_owned(),
                        from.max(now.saturating_sub(MAX_INTERVAL)).to_string(),
                    )
                }),
            ),
            Filter::Keyword { name, value } => (
                self.duration.max(0) as u64,
                Duration::from_secs(1),
                Some((name, value)),
            ),
            _ => (self.duration.max(0) as u64, Duration::from_secs(1), None),
        };
        let endpoint = self.endpoint(
            query
                .as_ref()
                .map(|(name, value)| (name.as_str(), value.as_str())),
        )?;

        let started = Instant::now();
        let mut stats = Stats::default();
        let poll = self.poll(&endpoint, delay, &out, &mut stats);
        tokio::select! {
            _ = out.closed() => return Err(eyre!("OpenSky downstream receiver closed")),
            result = async {
                if seconds == 0 {
                    poll.await
                } else {
                    match timeout(Duration::from_secs(seconds), poll).await {
                        Ok(result) => result,
                        Err(_) => Ok(()),
                    }
                }
            } => result?,
        }
        stats.tm = started.elapsed().as_secs();
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

#[derive(Deserialize)]
struct Snapshot {
    time: i64,
    states: Option<Vec<IgnoredAny>>,
}

/// Sends the first nonempty snapshot for a timestamp and counts skipped snapshots.
async fn forward_snapshot(
    payload: String,
    cache: &Cache<i64, ()>,
    out: &Sender<String>,
    stats: &mut Stats,
) -> Result<()> {
    let snapshot: Snapshot = serde_json::from_str(&payload).map_err(|err| {
        eyre!(
            "invalid OpenSky snapshot at line {}, column {}",
            err.line(),
            err.column()
        )
    })?;
    if snapshot.states.is_none_or(|states| states.is_empty()) {
        stats.empty += 1;
    } else if cache.get(&snapshot.time).is_some() {
        stats.hits += 1;
    } else {
        let bytes = payload.len() as u64;
        out.send(payload + "\n").await?;
        cache.insert(snapshot.time, ());
        stats.pkts += 1;
        stats.bytes += bytes;
        stats.miss += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The configured route appends to the base path and query values are escaped.
    #[test]
    fn polling_endpoint_preserves_base_path_and_encodes_query() -> Result<()> {
        let source = OpenskyServer {
            base_url: "https://example.test/api".into(),
            get: "/states/own".into(),
            ..OpenskyServer::default()
        };
        let url = source.endpoint(Some(("icao24", "aa&bb")))?;
        assert_eq!(url.path(), "/api/states/own");
        assert_eq!(url.query(), Some("icao24=aa%26bb"));
        Ok(())
    }

    /// Typical array snapshots are forwarded once, while empty snapshots only update counters.
    #[tokio::test]
    async fn polling_forwards_unique_nonempty_snapshots() -> Result<()> {
        let payload = r#"{"time":1,"states":[["abc123"]]}"#;
        let cache = Cache::new(4);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let mut stats = Stats::default();
        for input in [
            payload,
            payload,
            r#"{"time":2,"states":null}"#,
            r#"{"time":3,"states":[]}"#,
        ] {
            forward_snapshot(input.into(), &cache, &tx, &mut stats).await?;
        }
        drop(tx);
        assert_eq!(rx.recv().await.unwrap(), format!("{payload}\n"));
        assert!(rx.recv().await.is_none());
        assert_eq!(
            (stats.pkts, stats.hits, stats.miss, stats.empty),
            (1, 1, 1, 2)
        );
        assert_eq!(stats.bytes, payload.len() as u64);
        Ok(())
    }
}
