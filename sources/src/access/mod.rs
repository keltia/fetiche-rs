use std::fmt::{Display, Formatter};

use serde::Serialize;

pub use error::*;

#[cfg(feature = "aeroscope")]
pub use aeroscope::*;
#[cfg(feature = "asd")]
pub use asd::*;
#[cfg(feature = "avionix")]
pub use avionix::*;
#[cfg(feature = "flightaware")]
pub use flightaware::*;
#[cfg(feature = "opensky")]
pub use opensky::*;
#[cfg(feature = "safesky")]
pub use safesky::*;
#[cfg(feature = "senhive")]
pub use senhive::*;

mod error;

#[cfg(feature = "aeroscope")]
mod aeroscope;
#[cfg(feature = "asd")]
mod asd;
#[cfg(feature = "avionix")]
mod avionix;
#[cfg(feature = "flightaware")]
mod flightaware;
#[cfg(feature = "opensky")]
mod opensky;
#[cfg(feature = "safesky")]
mod safesky;
#[cfg(feature = "senhive")]
mod senhive;

/// Statistics gathering struct, should be generic enough for most sources
///
#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct Stats {
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
