//! Benchmark between dateparser, humantime and jiff.
//!
//! Apple Silicon M1 Max 3.2 GHz, Sonoma 14.5
//!
//! ```text
//! dateparser              time:   [1.0021 µs 1.0045 µs 1.0070 µs]
//!                         change: [−1.1123% −0.6057% −0.1310%] (p = 0.02 < 0.05)
//!
//! humantime               time:   [14.663 ns 14.698 ns 14.736 ns]
//!                         change: [+0.1197% +0.4958% +0.8872%] (p = 0.01 < 0.05)
//! Found 2 outliers among 100 measurements (2.00%)
//!   1 (1.00%) high mild
//!   1 (1.00%) high severe
//!
//! jiff                    time:   [15.626 ns 15.678 ns 15.731 ns]
//!                         change: [−0.0269% +0.2954% +0.6186%] (p = 0.07 > 0.05)
//!
//! jiff_timestamp          time:   [24.565 ns 24.623 ns 24.683 ns]
//!                         change: [−0.1266% +0.2607% +0.6665%] (p = 0.21 > 0.05)
//! Found 5 outliers among 100 measurements (5.00%)
//!   2 (2.00%) high mild
//!   3 (3.00%) high severe
//! ```
//!
use std::hint::black_box;

use chrono::Utc;
use criterion::{criterion_group, criterion_main, Criterion};
use jiff::civil::DateTime;
use jiff::Timestamp;

fn test_humantime(c: &mut Criterion) {
    let base = "2024-03-08 12:34:56";
    let mut curr = Utc::now();

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

fn test_dateparser(c: &mut Criterion) {
    let base = "2024-03-08 12:34:56";
    let mut curr = Utc::now();

    c.bench_function("dateparser", |b| {
        b.iter(|| {
            curr = black_box(dateparser::parse(base).unwrap());
        })
    });
    let _ = curr;
}

criterion_group!(
    benches,
    test_dateparser,
    test_humantime,
    test_jiff,
    test_jiff_timestamp
);
criterion_main!(benches);
