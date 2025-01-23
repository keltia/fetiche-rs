//! Module to deal with different kind of sources we can connect to to fetch data.
//!
//! The different submodules deal with the differences between sources:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!

use std::fmt::{Debug, Display};
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
pub use capability::*;
pub use error::*;
pub use filter::*;
pub use flow::*;
pub use init::*;
pub use route::*;
pub use site::*;
pub use sources::*;
pub use stats::*;

mod access;
pub mod actors;
mod auth;
mod capability;
mod error;
mod filter;
mod flow;
mod init;
mod route;
mod site;
mod sources;
mod stats;

#[macro_use]
mod macros;

/// A trait representing an entity that holds a key and can expire.
///
/// The `Expirable` trait provides two essential methods:
/// - [`key`]: Retrieves the unique identifier or "key" for the entity.
/// - [`is_expired`]: Checks whether the entity is expired.
///
/// This trait can be used for managing credentials, tokens, or other
/// expirable resources.
///
/// # Example
///
/// ```rust
/// use fetiche_sources::Expirable;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct MyToken {
///     key: String,
///     expiration: u64, // Epoch timestamp
/// }
///
/// impl Expirable for MyToken {
///     fn key(&self) -> String {
///         self.key.clone()
///     }
///
///     fn is_expired(&self) -> bool {
///         let current_time = 1681234567; // Example current timestamp
///         self.expiration < current_time
///     }
/// }
///
/// let token = MyToken {
///     key: String::from("my_unique_token"),
///     expiration: 1681234000,
/// };
///
/// println!("Token Key: {}", token.key());
/// println!("Is Expired: {}", token.is_expired());
/// ```
///
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

/// This is the enum used to do static dispatch (as opposed to the dynamic one of `Flow`).
///
#[enum_dispatch]
#[derive(Debug)]
pub enum Source {
    #[cfg(feature = "asd")]
    AsdFetch(Asd),
    #[cfg(feature = "aeroscope")]
    AeroscopeFetch(Aeroscope),
    #[cfg(feature = "avionix")]
    AvionixCubeAsyncStream(AvionixCube),
    #[cfg(feature = "avionix")]
    AvionixServerAsyncStream(AvionixServer),
    #[cfg(feature = "flightaware")]
    FlightawareAsyncStream(Flightaware),
    #[cfg(feature = "opensky")]
    OpenskyAsyncStream(Opensky),
    #[cfg(feature = "safesky")]
    SafeskyFetch(Safesky),
    #[cfg(feature = "senhive")]
    SenhiveAsyncStream(Senhive),
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
