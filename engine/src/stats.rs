//! All about `Stats`.

use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::ops::Add;

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
/// use fetiche_engine::Stats;
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

impl Add for Stats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Stats {
            tm: rhs.tm,
            pkts: self.pkts + rhs.pkts,
            reconnect: self.reconnect + rhs.reconnect,
            bytes: self.bytes + rhs.bytes,
            hits: self.hits + rhs.hits,
            miss: self.miss + rhs.miss,
            empty: self.empty + rhs.empty,
            err: self.err + rhs.err,
        }
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

    #[test]
    fn test_stats_add() {
        let stats1 = Stats {
            tm: 100,
            pkts: 1000,
            reconnect: 2,
            bytes: 5000,
            hits: 800,
            miss: 200,
            empty: 50,
            err: 5,
        };
        let stats2 = Stats {
            tm: 200,
            pkts: 2000,
            reconnect: 3,
            bytes: 7000,
            hits: 1500,
            miss: 300,
            empty: 100,
            err: 10,
        };
        let sum = stats1 + stats2;
        assert_eq!(sum.tm, 200);
        assert_eq!(sum.pkts, 3000);
        assert_eq!(sum.reconnect, 5);
        assert_eq!(sum.bytes, 12000);
        assert_eq!(sum.hits, 2300);
        assert_eq!(sum.miss, 500);
        assert_eq!(sum.empty, 150);
        assert_eq!(sum.err, 15);
    }

    #[test]
    fn test_stats_add_zero() {
        let stats1 = Stats {
            tm: 100,
            pkts: 1000,
            reconnect: 2,
            bytes: 5000,
            hits: 800,
            miss: 200,
            empty: 50,
            err: 5,
        };
        let stats2 = Stats::default();
        let sum = stats1.clone() + stats2;
        assert_eq!(sum.tm, 0);
        assert_eq!(sum.pkts, stats1.pkts);
        assert_eq!(sum.reconnect, stats1.reconnect);
        assert_eq!(sum.bytes, stats1.bytes);
        assert_eq!(sum.hits, stats1.hits);
        assert_eq!(sum.miss, stats1.miss);
        assert_eq!(sum.empty, stats1.empty);
        assert_eq!(sum.err, stats1.err);
    }

    #[test]
    fn test_stats_add_tm_from_rhs() {
        let stats1 = Stats {
            tm: 100,
            ..Default::default()
        };
        let stats2 = Stats {
            tm: 200,
            ..Default::default()
        };
        let sum = stats1 + stats2;
        assert_eq!(sum.tm, 200);
    }
}

