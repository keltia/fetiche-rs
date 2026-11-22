//! Benchmark between dateparser, humantime and jiff.
//!
//! AMD 7700X, 32 GB, Rust 1.100.0, jiff 1.0.2
//! ```text
//! humantime               time:   [8.6784 ns 8.6899 ns 8.7025 ns]
//!                         change: [−0.1621% +1.4839% +3.4864%] (p = 0.10 > 0.05)
//!
//! jiff                    time:   [15.895 ns 15.938 ns 15.985 ns]
//!                         change: [−0.3307% +0.2600% +0.9299%] (p = 0.46 > 0.05)
//!
//! jiff_timestamp          time:   [23.915 ns 23.973 ns 24.029 ns]
//!                         change: [−1.0791% −0.3673% +0.0863%] (p = 0.28 > 0.05)
//! ```
//!
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use jiff::Timestamp;
use jiff::civil::DateTime;

fn test_humantime(c: &mut Criterion) {
    let base = "2024-03-08 12:34:56";
    let mut curr = std::time::SystemTime::UNIX_EPOCH;

    c.bench_function("humantime", |b| {
        b.iter(|| {
            let this = black_box(humantime::parse_rfc3339_weak(base).unwrap());
            curr = this.into();
        })
    });
    let _ = curr;
}

fn test_jiff(c: &mut Criterion) {
    let base = "2024-03-08 12:34:56";
    let mut curr: DateTime = DateTime::ZERO;

    c.bench_function("jiff", |b| {
        b.iter(|| {
            curr = black_box(base.parse().unwrap());
        })
    });
}

fn test_jiff_timestamp(c: &mut Criterion) {
    let base = "2024-03-08T12:34:56Z";
    let mut curr = Timestamp::now();

    c.bench_function("jiff_timestamp", |b| {
        b.iter(|| {
            curr = black_box(base.parse().unwrap());
        })
    });
}

criterion_group!(benches, test_humantime, test_jiff, test_jiff_timestamp);
criterion_main!(benches);
