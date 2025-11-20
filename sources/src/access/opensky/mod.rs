//! OpenSky (.org) specific code
//!
//! There are two trait implementations:
//! - `Fetchable`
//! - `Streamable`
//!
//! The `/states/own` endpoint can be polled several times and it always return a specific
//! `StateList` for which `time` is both timestamp and index.
//!
//! if two `StateList`s have the same `time`, there are the same.
//!
//! So now we cache them.
//!
//! FIXME: use a similar pattern as Senhive with actors.

mod device;
mod server;

use std::str::FromStr;
use std::time::Duration;

use mini_moka::sync::ConcurrentCacheExt;
use serde::{Deserialize, Serialize};

use crate::Site;
use crate::{Auth, Capability, StreamableSource};

pub use device::*;
pub use server::*;

/// Messages to send to the stats threads
///
#[derive(Clone, Debug, Serialize)]
enum StatMsg {
    Pkts,
    Bytes(u64),
    Hits,
    Miss,
    Empty,
    Error,
    Print,
    Exit,
}

/// We can go back only 1h in Opensky API
const MAX_INTERVAL: i64 = 3600;

/// Expiration after insert/get
pub const CACHE_IDLE: Duration = Duration::from_secs(20);
/// Expiration after insert
pub const CACHE_MAX: Duration = Duration::from_secs(60);
/// Cache max entries
pub const CACHE_SIZE: u64 = 20;

