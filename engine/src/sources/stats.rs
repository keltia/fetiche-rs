//! All about `Stats`.

use serde::Serialize;
use std::fmt::{Display, Formatter};

/// `Stats` is a structure used to track various performance-related statistics
/// for data sources in the system.
///
/// This struct consolidates a variety of metrics, such as traffic information,
/// reconnection attempts, and error counts, which are useful for monitoring and
/// debugging purposes.
///
/// # Fields
///
/// - `tm`: The total elapsed time in seconds since the monitoring began.
/// - `pkts`: The number of packets processed.
/// - `reconnect`: The total number of reconnection attempts.
/// - `bytes`: The total number of bytes processed.
/// - `hits`: The number of successful requests or accesses.
/// - `miss`: The number of failed requests or cache misses.
/// - `empty`: The number of empty or null responses.
/// - `err`: The number of errors encountered during operation.
///
/// # Example
///
/// ```rust
/// use fetiche_sources::Stats;
///
/// let stats = Stats {
///     tm: 3600,
///     pkts: 3456,
///     reconnect: 3,
///     bytes: 987654,
///     hits: 1200,
///     miss: 200,
///     empty: 50,
///     err: 15,
/// };
///
/// println!("Stats summary: {}", stats);
/// ```
///
/// This example demonstrates how to create an instance of `Stats` and display
/// it using its `Display` implementation.
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_default() {
        let stats = Stats::default();
        assert_eq!(stats.tm, 0);
        assert_eq!(stats.pkts, 0);
        assert_eq!(stats.reconnect, 0);
        assert_eq!(stats.bytes, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.miss, 0);
        assert_eq!(stats.empty, 0);
        assert_eq!(stats.err, 0);
    }

    #[test]
    fn test_stats_display() {
        let stats = Stats {
            tm: 3600,
            pkts: 3456,
            reconnect: 3,
            bytes: 987654,
            hits: 1200,
            miss: 200,
            empty: 50,
            err: 15,
        };
        let display = format!("{}", stats);
        assert_eq!(
            display,
            "time=3600s pkts=3456 bytes=987654 reconnect=3 hits=1200 miss=200 empty=50 errors=15"
        );
    }

    #[test]
    fn test_stats_custom_values() {
        let stats = Stats {
            tm: 100,
            pkts: 1500,
            reconnect: 5,
            bytes: 2048,
            hits: 1000,
            miss: 500,
            empty: 100,
            err: 10,
        };
        assert_eq!(stats.tm, 100);
        assert_eq!(stats.pkts, 1500);
        assert_eq!(stats.reconnect, 5);
        assert_eq!(stats.bytes, 2048);
        assert_eq!(stats.hits, 1000);
        assert_eq!(stats.miss, 500);
        assert_eq!(stats.empty, 100);
        assert_eq!(stats.err, 10);
    }
}

