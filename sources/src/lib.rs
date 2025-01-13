//! Module to deal with different kind of sources we can connect to to fetch data.
//!
//! The different submodules deal with the differences between sources:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!

use std::fmt::{Debug, Display, Formatter};
use std::sync::mpsc::Sender;

use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use eyre::Result;
use serde::{Deserialize, Serialize};

use fetiche_formats::Format;

// Re-export these modules for a shorted import path.
//
pub use access::*;
pub use auth::*;
pub use error::*;
pub use filter::*;
pub use route::*;
pub use site::*;
pub use sources::*;

mod access;
pub mod actors;
mod auth;
mod error;
mod filter;
mod route;
mod site;
mod sources;

#[macro_use]
mod macros;

#[enum_dispatch(TokenType)]
pub trait Expirable: Debug + Clone {
    fn key(&self) -> String;
    fn is_expired(&self) -> bool;
}

#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenType {
    AsdToken(AsdToken),
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Capability {
    #[default]
    None = 0,
    Fetch = 1,
    Read = 2,
    Stream = 3,
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Capability::None => "none",
            Capability::Read => "read",
            Capability::Fetch => "fetch",
            Capability::Stream => "stream",
        };
        write!(f, "{s}")
    }
}

/// Statistics gathering struct, should be generic enough for most sources
///
#[derive(Clone, Debug, Default, Serialize)]
pub struct Stats {
    pub tm: u64,
    pub pkts: u32,
    pub reconnect: usize,
    pub bytes: u64,
    pub hits: u32,
    pub miss: u32,
    pub empty: u32,
    pub err: u32,
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "time={}s pkts={} bytes={} reconnect={} hits={} miss={} empty={} errors={}",
            self.tm,
            self.pkts,
            self.bytes,
            self.reconnect,
            self.hits,
            self.miss,
            self.empty,
            self.err
        )
    }
}

/// We have three different traits now
///
#[derive(Debug)]
pub enum Flow {
    Fetchable(Box<dyn Fetchable>),
    Streamable(Box<dyn Streamable>),
    AsyncStreamable(Box<dyn AsyncStreamable>),
}

impl Flow {
    /// Return the name of the underlying object
    ///
    #[inline]
    pub fn name(&self) -> String {
        match self {
            Flow::Fetchable(s) => s.name(),
            Flow::Streamable(s) => s.name(),
            Flow::AsyncStreamable(s) => s.name(),
        }
    }

    /// Return the format of the underlying object
    ///
    #[inline]
    pub fn format(&self) -> Format {
        match self {
            Flow::Fetchable(s) => s.format(),
            Flow::Streamable(s) => s.format(),
            Flow::AsyncStreamable(s) => s.format(),
        }
    }
}

/// This trait enables us to manage different ways of connecting and fetching data under
/// a single interface.
///
pub trait Fetchable: Debug {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String, AuthError>;
    /// Fetch actual data
    fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> Result<()>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// This trait enables us to manage different ways of connecting and streaming data under
/// a single interface.  The object can connect to a TCP stream or create one by repeatedly calling
/// some API (cf. Opensky).
///
pub trait Streamable: Debug {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String, AuthError>;
    /// Stream actual data
    fn stream(&self, out: Sender<String>, token: &str, args: &str) -> Result<()>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// This trait enables us to manage different ways of connecting and streaming data under
/// a single interface.  The object can connect to a TCP stream or create one by repeatedly calling
/// some API (cf. Opensky).
///
/// This is the async version of `Streamable`, making it easier to use async clients and/or actors.
///
#[async_trait]
pub trait AsyncStreamable: Debug {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    async fn authenticate(&self) -> Result<String, AuthError>;
    /// Stream actual data
    async fn stream(&self, out: Sender<String>, token: &str, args: &str) -> Result<()>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// Default configuration filename
const CONFIG: &str = "sources.hcl";

pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}
