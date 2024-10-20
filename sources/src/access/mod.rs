pub use aeroscope::*;
pub use asd::*;
pub use avionix::*;
pub use flightaware::*;
pub use opensky::*;
pub use safesky::*;
use serde::Serialize;
use std::fmt::{Display, Formatter};

mod aeroscope;
mod asd;
mod avionix;
mod error;
mod flightaware;
mod opensky;
mod safesky;

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
