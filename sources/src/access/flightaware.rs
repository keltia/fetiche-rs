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

use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::mpsc::Sender;

use eyre::{eyre, Result};
use native_tls::TlsConnector;
use serde::{Deserialize, Serialize};
use strum::{EnumString, EnumVariantNames};
use tracing::trace;

use fetiche_formats::Format;

use crate::{Auth, Capability, Fetchable, Site, Streamable};

/// This si the Flightaware client/source struct.
///
#[derive(Clone, Debug)]
pub struct Flightaware {
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
    pub pitr: Option<u32>,
    /// Time to start from
    pub start: Option<String>,
    /// Time to stop to
    pub end: Option<String>,
    /// Compression type — **UNSUPPORTED**
    pub compress: Option<Compress>,
    /// Events
    pub events: Vec<Events>,
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
    #[tracing::instrument]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("flightaware::load");

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
}

/// Small helper function
///
fn get_timestamp(date: Option<String>) -> Result<i64> {
    let date = date.unwrap();
    let date = dateparser::parse(&date).unwrap();
    Ok(date.timestamp())
}

impl Fetchable for Flightaware {
    fn name(&self) -> String {
        String::from("flightaware")
    }

    /// Credentials are passed in the call the API    
    ///
    #[tracing::instrument]
    fn authenticate(&self) -> Result<String> {
        trace!("fake auth");

        Ok(format!("{}:{}", self.login, self.password))
    }

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
        let (start, end) = if args.start.is_some() && args.end.is_some() {
            (get_timestamp(args.start)?, get_timestamp(args.end)?)
        } else {
            return Err(eyre!("No start and/or end, use stream."));
        };

        // Build the request string
        //
        let req = format!(
            "username {} password {} range {} {} events \"position\"",
            self.login, self.password, start, end
        );

        // Setup TLS connection
        //
        let connector = TlsConnector::new().unwrap();
        let stream = TcpStream::connect(&self.base_url).unwrap();
        let mut stream = connector.connect("flightaware.com", stream).unwrap();

        // Send request
        //
        stream.write_all(req.as_bytes())?;

        // Get answer
        //
        let mut res = String::new();
        stream.read_to_string(&mut res)?;

        Ok(out.send(res)?)
    }

    fn format(&self) -> Format {
        Format::Flightaware
    }
}

impl Streamable for Flightaware {
    fn name(&self) -> String {
        todo!()
    }

    /// All credentials are passed every time we call the API so return a fake token
    ///
    #[tracing::instrument]
    fn authenticate(&self) -> Result<String> {
        trace!("fake auth");
        Ok(format!("{}:{}", self.login, self.password))
    }

    fn stream(&self, out: Sender<String>, token: &str, args: &str) -> Result<()> {
        todo!()
    }

    fn format(&self) -> Format {
        Format::Flightaware
    }
}
