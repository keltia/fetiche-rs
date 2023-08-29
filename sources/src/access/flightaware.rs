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
//! There is not much differences between `Fetch` and `Stream` due to nature of FA's API.  One always
//! open up a TLS connection to the site and send a request.  If this is a `live` or `pitr` one you
//! get a stream and `range` gets you a "fixed" stream.
//!

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::mpsc::Sender;

use base64_light::base64_encode;
use eyre::{eyre, Result};
use native_tls::{TlsConnector, TlsStream};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use strum::{EnumString, EnumVariantNames};
use tracing::trace;

use fetiche_formats::Format;

use crate::{version, Auth, Capability, Fetchable, Site, Streamable};

/// Firehose is out target
const SITE: &str = "firehose.flightaware.com";
/// Standard FA port
const PORT: u16 = 1501;

/// This si the Flightaware client/source struct.
///
#[derive(Clone, Debug)]
pub struct Flightaware {
    /// Name of the source
    pub name: String,
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
    /// Add this to `base_url` to fetch data
    pub get: String,
    /// How to stream
    pub stream: String,
    /// Running time (for streams)
    pub duration: i32,
}

/// This is the struct holding potential parameters to the API
///
#[derive(Debug, Deserialize, Serialize)]
struct Param {
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

#[derive(Debug, Deserialize, strum::Display, EnumString, EnumVariantNames, Serialize)]
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
#[derive(Debug, Default, Deserialize, strum::Display, EnumString, EnumVariantNames, Serialize)]
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
            features: vec![Capability::Fetch, Capability::Stream],
            format: Format::Flightaware,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            stream: "".to_owned(),
            duration: 0,
        }
    }

    /// Load some data from in-memory loaded config
    ///
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("flightaware::load");

        self.name = site.name.as_ref().unwrap().to_owned();
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
    pub fn connect(&self, proxy: Option<String>) -> Result<TlsStream<TcpStream>> {
        let connector = TlsConnector::new()?;

        // FIXME: this only support HTTP proxy, not HTTPS nor SOCKS
        //
        let stream = if proxy.is_some() {
            trace!("using proxy");

            let url = Url::parse(&proxy.unwrap())?;
            let (host, port) = (url.host().unwrap(), url.port().unwrap());

            trace!("proxy = {}:{}", host, port);

            let username = url.username();
            let passwd = url.password().unwrap_or("");

            // base64_light API is better.
            //
            let auth = base64_encode(&format!("{}:{}", username, passwd));
            trace!("Auth token is {}", auth);

            trace!("CONNECT");
            let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;

            stream.write_all(
                format!(
                    r##"CONNECT {}:{} HTTP/1.1
Proxy-Authorization: Basic {}
User-Agent: {}
Proxy-Connection: Keep-Alive

"##,
                    SITE,
                    PORT,
                    auth,
                    version()
                )
                .as_bytes(),
            )?;
            stream
        } else {
            trace!("no proxy");

            TcpStream::connect(format!("{}:{}", SITE, PORT))?
        };
        // Handover to the TLS engine hopefully
        //
        trace!("TCP={:?}", stream);

        let stream = connector.connect(SITE, stream)?;
        Ok(stream)
    }
}

/// Small helper function
///
#[tracing::instrument]
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
    fn authenticate(&self) -> Result<String> {
        trace!("fake auth");

        Ok(format!("{}:{}", self.login, self.password))
    }

    #[tracing::instrument(skip(self))]
    fn fetch(&self, out: Sender<String>, _token: &str, args: &str) -> Result<()> {
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
        let mut stream = self.connect(proxy)?;

        // Send request
        //
        trace!("req={req}");
        stream.write_all(req.as_bytes())?;

        trace!("read answer");

        let buf = BufReader::new(&mut stream);
        for line in buf.lines() {
            let line = line.unwrap();
            trace!("line={}", line);
            let _ = out.send(line);
        }

        Ok(())
    }

    fn format(&self) -> Format {
        Format::Flightaware
    }
}

impl Streamable for Flightaware {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    /// All credentials are passed every time we call the API so return a fake token
    ///
    #[tracing::instrument]
    fn authenticate(&self) -> Result<String> {
        trace!("fake auth");
        Ok(format!("{}:{}", self.login, self.password))
    }

    fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<()> {
        trace!("stream with TLS");
        let args: Param = serde_json::from_str(args)?;

        // Check arguments
        //
        let cmd = match args.pitr {
            Some(pitr) => Command::Pitr { pitr },
            None => Command::Live,
        };

        let req = self.request(cmd)?;

        // Setup TLS connection, check proxy environment var first.
        //
        let proxy = match std::env::var("http_proxy") {
            Ok(s) => Some(s),
            Err(_) => None,
        };

        trace!("tls connect");
        let mut stream = self.connect(proxy)?;

        // Send request
        //
        trace!("req={req}");
        stream.write_all(req.as_bytes())?;

        trace!("read answer");

        let buf = BufReader::new(&mut stream);
        for line in buf.lines() {
            let line = line.unwrap();
            trace!("line={}", line);
            let _ = out.send(line);
        }

        Ok(())
    }

    fn format(&self) -> Format {
        Format::Flightaware
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    #[test]
    fn test_get_timestamp() {
        let t = get_timestamp(Some("2023-08-02T00:00:00Z".to_string()));
        let d = Utc.with_ymd_and_hms(2023, 8, 2, 0, 0, 0).unwrap();

        assert!(t.is_ok());
        assert_eq!(d.timestamp(), t.unwrap());
    }
}
