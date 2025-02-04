//! Module to deal with different kind of sources we can connect to fetch data.
//!
//! The different submodules deal with the differences between sources:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!

use std::fmt::Debug;
use std::sync::mpsc::Sender;

use enum_dispatch::enum_dispatch;
use eyre::Result;
use serde::{Deserialize, Serialize};

use fetiche_formats::Format;

pub use crate::stats::*;

// Re-export these modules for a shorted import path.
//
pub use access::*;
pub use capability::*;
pub use config::*;
pub use error::*;
pub use route::*;
pub use site::*;

mod access;
mod capability;
mod config;
mod error;
mod route;
mod site;

#[macro_use]
mod macros;

#[enum_dispatch]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum FetchableSource {
    Asd,
    #[cfg(feature = "aeroscope")]
    Aeroscope,
    #[cfg(feature = "safesky")]
    Safesky,
}

impl From<Site> for FetchableSource {
    fn from(value: Site) -> Self {
        match value.format.as_str() {
            #[cfg(feature = "asd")]
            "asd" => {
                let s = Asd::new().load(&value).clone();
                FetchableSource::from(s)
            }
            #[cfg(feature = "aeroscope")]
            "aeroscope" => {
                let s = Aeroscope::new().load(&value).clone();
                FetchableSource::from(s)
            }
            #[cfg(feature = "safesky")]
            "safesky" => {
                let s = Safesky::new().load(&value).clone();
                FetchableSource::from(s)
            }
            _ => unimplemented!(),
        }
    }
}

#[enum_dispatch]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum StreamableSource {
    #[cfg(feature = "avionix")]
    Cube,
    #[cfg(feature = "avionix")]
    AvionixServer,
    #[cfg(feature = "flightaware")]
    Flightaware,
    #[cfg(feature = "opensky")]
    Opensky,
    #[cfg(feature = "senhive")]
    Senhive,
}

impl From<Site> for StreamableSource {
    fn from(value: Site) -> Self {
        match value.format.as_str() {
            #[cfg(feature = "avionix")]
            "avionixcube" => {
                let s = Cube::new().load(&value).clone();
                StreamableSource::from(s)
            }
            #[cfg(feature = "avionix")]
            "avionixserver" => {
                let s = AvionixServer::new().load(&value).clone();
                StreamableSource::from(s)
            }
            #[cfg(feature = "flightaware")]
            "flightaware" => {
                let s = Flightaware::new().load(&value).clone();
                StreamableSource::from(s)
            }
            #[cfg(feature = "senhive")]
            "senhive" => {
                let s = Senhive::new().load(&value).clone();
                StreamableSource::from(s)
            }
            #[cfg(feature = "opensky")]
            "opensky" => {
                let s = Opensky::new().load(&value).clone();
                StreamableSource::from(s)
            }
            _ => unimplemented!(),
        }
    }
}

/// This trait enables us to manage different ways of connecting and fetching data under
/// a single interface.
///
/// This is the async version of `Fetchable`, making it easier to use async clients and/or actors.
///
#[allow(async_fn_in_trait)]
#[enum_dispatch(FetchableSource)]
pub trait Fetchable {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    async fn authenticate(&self) -> Result<String, AuthError>;
    /// Stream actual data
    async fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> Result<Stats>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// This trait enables us to manage different ways of connecting and streaming data under
/// a single interface.  The object can connect to a TCP stream or create one by repeatedly calling
/// some API (cf. Opensky).
///
/// This is the async version of `Streamable`, making it easier to use async clients and/or actors.
///
#[allow(async_fn_in_trait)]
#[enum_dispatch(StreamableSource)]
pub trait Streamable: Debug {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    async fn authenticate(&self) -> Result<String, AuthError>;
    /// Stream actual data
    async fn stream(&self, out: Sender<String>, token: &str, args: &str) -> Result<Stats>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// Default configuration filename
pub const SOURCES_CONFIG: &str = "sources.hcl";
