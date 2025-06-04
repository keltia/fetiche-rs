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
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Sender};
use tracing::{info, trace};

use fetiche_formats::{Format, StateList};

use crate::Site;
use crate::{Auth, Capability, StreamableSource};

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

