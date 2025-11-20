//! Flightaware connection & client code
//!
//! There are two trait implementations:
//! - `Fetchable`
//! - `Streamable`
//!
//! In Firehose, there is no concept of REST API or anything, just a TCP/TLS pipe you feed
//! commands into and get a result back.  This can be a continuous stream (`live`), a single
//! bounced one (restarting through `pitr`) then live or just a stream of data corresponding to
//! a time-bound request (`range`).
//!
//! For now, the only event wwe are supporting at this level is `Position`, an ADS-B airplane
//! position in time and space.  Again, this is not a general FA access library.
//!
//! There is not many differences between `Fetch` and `Stream` due to nature of FA's API.  One always
//! open up a TLS connection to the site and send a request.  If this is a `live` or `pitr` one you
//! get a stream and `range` gets you a "fixed" stream.
//!

mod stream;

use std::io::{BufRead, BufReader, Write};
use std::str::FromStr;
use std::sync::mpsc::Sender;

use base64_light::base64_encode;
use eyre::{eyre, Result};
use ractor::ActorRef;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use strum::EnumString;
use strum::VariantNames;
// Add these imports
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_native_tls::TlsConnector as TokioTlsConnector;
use tracing::trace;

use fetiche_formats::Format;

use crate::actors::StatsMsg;
use crate::{version, AccessError, Auth, AuthError, Capability, Fetchable, Site, Stats, Streamable, StreamableSource};

/// Firehose is out target
const SITE: &str = "firehose.flightaware.com";
/// Standard FA port
const PORT: u16 = 1501;

/// This si the Flightaware client/source struct.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Flightaware {
    /// Name of the source
    pub name: String,
    /// Describe the different features of the source
    pub feature: Capability,
    /// Input formats
    pub format: Format,
    /// Username
    pub login: String,
    /// Password
    pub password: String,
    /// Base site url taken from config
    pub base_url: String,
    /// Add this to `base_url` to fetch data
    pub get: String,
    /// How to stream
    pub stream: String,
    /// Running time (for streams)
    pub duration: i32,
    #[serde(skip)]
    pub stat: Option<ActorRef<StatsMsg>>,
}

/// This is the struct holding potential parameters to the API
///
#[derive(Debug, Deserialize, Serialize)]
pub struct Param {
    /// timestamp of the state vectors to be retrieved
    pub pitr: Option<i64>,
    /// Time to start from
    pub begin: Option<String>,
    /// Time to stop to
    pub end: Option<String>,
    /// Compression type — **UNSUPPORTED**
    pub compress: Option<Compress>,
    /// Events
    pub events: Option<Vec<Events>>,
}

#[derive(Debug, Deserialize, strum::Display, EnumString, VariantNames, Serialize)]
#[strum(serialize_all = "lowercase")]
pub enum Compress {
    Compress,
    Deflate,
    Gzip,
}

/// Different events one can request from Firehose.  We use only Position
///
/// see `formats/src/flightaware/mod.rs` for details
///
#[derive(Debug, Default, Deserialize, strum::Display, EnumString, VariantNames, Serialize)]
#[strum(serialize_all = "snake_case")]
pub enum Events {
    // Airborne
    Arrival,
    Cancellation,
    Departure,
    FlightPlan,
    ExtendedFlightInfo,
    Flifo,
    SurfaceOffblock,
    SurfaceOnblock,
    PowerOn,
    #[default]
    Position,
    // Surface
    GroundPosition,
    VehiculePosition,
    NearSurfacePosition,
    LocationEntry,
    LocationExit,
    // Weather
    Fmswx,
}

#[derive(Debug)]
pub enum Command {
    Live,
    Pitr { pitr: i64 },
    Range { begin: i64, end: i64 },
}

/// Credentials to submit to the site to get the token
///
#[derive(Debug, Deserialize, Serialize)]
struct Credentials {
    /// Email as username
    username: String,
    /// Password
    password: String,
}

impl Flightaware {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("flightaware::new");

        Flightaware {
            name: "".to_owned(),
            feature: Capability::Stream,
            format: Format::Flightaware,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            stream: "".to_owned(),
            duration: 0,
            stat: None,
        }
    }

    /// Load some data from in-memory loaded config
    ///
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("flightaware::load");

        self.name = site.name.clone();
        self.format = Format::from_str(&site.format).unwrap();
        self.base_url = site.base_url.to_owned();
        if let Some(auth) = &site.auth {
            match auth {
                Auth::Login {
                    username: login,
                    password,
                } => {
                    self.login = login.to_owned();
                    self.password = password.to_owned();
                }
                _ => panic!("nope"),
            }
        }
        // FIXME: should get the entire set of routes
        //
        self.get = site.route("get").unwrap().to_owned();
        self.stream = site.route("stream").unwrap().to_owned();
        self
    }

    /// Generate the proper command string
    ///
    #[tracing::instrument(skip(self))]
    fn request(&self, cmd: Command) -> Result<String> {
        let str = match cmd {
            Command::Live => format!(
                "live username {} password {} events \"position\"\n",
                self.login, self.password
            ),
            Command::Pitr { pitr } => format!(
                "pitr {} username {} password {} events \"position\"\n",
                pitr, self.login, self.password
            ),
            Command::Range { begin, end } => format!(
                "range {} {} username {} password {} events \"{}\"\n",
                begin, end, self.login, self.password, "position"
            ),
        };
        Ok(str)
    }

    /// Establish the TCP/TLS connection, optionally goes through an HTTP proxy
    ///
    #[tracing::instrument(skip(self))]
    async fn connect(&self, proxy: Option<String>) -> Result<TlsStream<TcpStream>> {
        let native_connector = native_tls::TlsConnector::new()?;
        let connector = TokioTlsConnector::from(native_connector);

        // FIXME: this only support HTTP proxy, not HTTPS nor SOCKS
        //
        let stream = if let Some(proxy_url_str) = proxy {
            trace!("using proxy");

            let url = Url::parse(&proxy_url_str).map_err(|e| AccessError::BadProxyString(proxy_url_str).into())?;
            let (host, port) = (url.host_str().unwrap(), url.port().unwrap_or(80));

            trace!("proxy = {}:{}", host, port);

            // Connect to the proxy server
            let mut stream = TcpStream::connect(format!("{}:{}", host, port)).await?;

            // If proxy requires authentication
            let auth = if !url.username().is_empty() {
                let passwd = url.password().unwrap_or("");
                let credentials = format!("{}:{}", url.username(), passwd);
                format!("Proxy-Authorization: Basic {}\r\n", base64_light::base64_encode(&credentials))
            } else {
                String::new()
            };

            // Send the CONNECT request to the proxy
            let connect_req = format!(
                "CONNECT {}:{} HTTP/1.1\r\nHost: {}\r\n{}User-Agent: {}\r\n\r\n",
                SITE, PORT, SITE, auth, version()
            );

            stream.write_all(connect_req.as_bytes()).await?;

            // Read the proxy's response
            let mut buf = vec![0; 1024];
            let n = stream.read(&mut buf).await?;
            let response = String::from_utf8_lossy(&buf[..n]);

            if !response.starts_with("HTTP/1.1 200") {
                return Err(AccessError::ProxyConnectFailed.into());
            }

            stream
        } else {
            trace!("no proxy");
            TcpStream::connect(format!("{}:{}", SITE, PORT)).await?
        };

        // Handover to the TLS engine
        trace!("TCP={:?}", stream);
        let tls_stream = connector.connect(SITE, stream)
            .await
            .map_err(|e| AccessError::TlsConnectFailed(e).into())?;

        Ok(tls_stream)
    }


    #[tracing::instrument(skip(self, stat))]
    pub fn stats(&mut self, stat: ActorRef<StatsMsg>) -> &mut Self {
        self.stat = Some(stat);
        self
    }

    pub fn source(&self) -> StreamableSource {
        StreamableSource::Flightaware(self.clone())
    }
}

/// Small helper function
///
#[inline]
fn get_timestamp(date: Option<String>) -> Result<i64> {
    let date = date.unwrap();
    trace!("date={date}");
    let date = dateparser::parse(&date).unwrap();
    Ok(date.timestamp())
}

impl Fetchable for Flightaware {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    /// Credentials are passed in the call the API    
    ///
    #[tracing::instrument(skip(self))]
    async fn authenticate(&self) -> Result<String, AuthError> {
        trace!("fake auth");

        Ok(format!("{}:{}", self.login, self.password))
    }

    #[tracing::instrument(skip(self, _token))]
    async fn fetch(&self, out: Sender<String>, _token: &str, args: &str) -> Result<()> {
        trace!("fetch with TLS");
        let args: Param = serde_json::from_str(args)?;

        // Check arguments
        //
        if args.pitr.is_some() {
            return Err(eyre!("Bad argument, 'pitr' is for streams"));
        }

        // Get the range parameters
        //
        let cmd = if args.begin.is_some() && args.end.is_some() {
            let (begin, end) = (get_timestamp(args.begin)?, get_timestamp(args.end)?);
            Command::Range { begin, end }
        } else {
            return Err(eyre!("No start and/or end, use stream."));
        };

        let req = self.request(cmd)?;

        // Setup TLS connection, check proxy environment var first.
        //
        let proxy = match std::env::var("http_proxy") {
            Ok(s) => Some(s),
            Err(_) => None,
        };

        trace!("tls connect");
        let mut stream = self.connect(proxy).await?;

        // Send request
        //
        trace!("req={req}");
        stream.write_all(req.as_bytes())?;

        trace!("read answer, format as an array");
        let buf = BufReader::new(&mut stream);
        let res = buf
            .lines()
            .map(|l| l.unwrap())
            .inspect(|l| {
                trace!("line={l}");
            })
            .collect::<Vec<_>>()
            .join(",\n");
        trace!("End of fetch");

        drop(stream);
        Ok(out.send(format!("[{res}]"))?)
    }

    fn format(&self) -> Format {
        Format::Flightaware
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_get_timestamp() {
        let t = get_timestamp(Some("2023-08-02T00:00:00Z".to_string()));
        let d = Utc.with_ymd_and_hms(2023, 8, 2, 0, 0, 0).unwrap();

        assert!(t.is_ok());
        assert_eq!(d.timestamp(), t.unwrap());
    }
}
