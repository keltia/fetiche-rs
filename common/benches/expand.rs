//! Benchmark for expand_interval between chrono/jiff
//!
//! AMD 7700X, 32 GB, Rust 1.100.0, jiff 1.0.2
//! ```text
//! jiff_loop               time:   [4.3455 µs 4.3606 µs 4.3771 µs]
//!                         change: [−0.0016% +0.3997% +0.8157%] (p = 0.05 > 0.05)
//!
//! jiff_iter               time:   [4.8453 µs 4.8558 µs 4.8679 µs]
//!                         change: [−0.2014% +0.1856% +0.6133%] (p = 0.40 > 0.05)
//!
//! jiff_series             time:   [7.6338 µs 7.6504 µs 7.6671 µs]
//!                         change: [−8.0658% −5.7577% −3.4942%] (p = 0.00 < 0.05)
//! ```

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use jiff::civil::DateTime;
use jiff::{Span, ToSpan};

pub fn expand_interval_jiff(begin: DateTime, end: DateTime) -> eyre::Result<Vec<DateTime>> {
    // Pre-calculate capacity: days between begin and end
    let days_span = end.since(begin)?;
    let days_count = days_span.get_days();

    // Pre-allocate with exact capacity
    let mut intv = Vec::with_capacity(days_count as usize);

    let day = Span::new().days(1);
    let mut d = begin;

    while d < end {
        intv.push(d);
        d = d.checked_add(day).expect("overflow");
    }
    Ok(intv)
}

pub fn expand_interval_jiff_iter(begin: DateTime, end: DateTime) -> eyre::Result<Vec<DateTime>> {
    // Use successors to chain single-day additions (only creates one Span)
    let day = Span::new().days(1);
    let dates: Vec<_> = std::iter::successors(Some(begin), |&d| {
        let next = d.checked_add(day).ok()?;
        (next < end).then_some(next)
    })
    .collect();

    Ok(dates)
}

fn expand_interval_series_jiff(begin: DateTime, end: DateTime) -> eyre::Result<Vec<DateTime>> {
    // Pre-calculate capacity to hint collect()
    let days_span = end.since(begin)?;
    let days_count = days_span.get_days() as usize;

    // series() is slower because:
    // 1. It's a generic iterator for any period (days/months/years/etc)
    // 2. More validation per step (overflow checks, calendar math)
    // 3. Cannot be optimized as aggressively by LLVM
    // 4. Iterator state machine overhead
    let mut intv = Vec::with_capacity(days_count);
    intv.extend(begin.series(1.days()).take(days_count));

    Ok(intv)
}

// -----

fn test_jiff(c: &mut Criterion) {
    let mut r = vec![];

    // 2024 has 366 days
    let begin = "2024-01-01".parse().unwrap();
    let end = "2025-01-01".parse().unwrap();

    c.bench_function("jiff_loop", |b| {
        b.iter(|| {
            r = black_box(expand_interval_jiff(begin, end).unwrap());
        })
    });
    assert_eq!(r.len(), 366);
}

fn test_jiff_series(c: &mut Criterion) {
    let mut r = vec![];

    // 2024 has 366 days
    let begin = "2024-01-01".parse().unwrap();
    let end = "2025-01-01".parse().unwrap();

    c.bench_function("jiff_series", |b| {
        b.iter(|| {
            r = black_box(expand_interval_series_jiff(begin, end).unwrap());
        })
    });
    assert_eq!(r.len(), 366);
}

fn test_jiff_iter(c: &mut Criterion) {
    let mut r = vec![];

    // 2024 has 366 days
    let begin = "2024-01-01".parse().unwrap();
    let end = "2025-01-01".parse().unwrap();

    c.bench_function("jiff_iter", |b| {
        b.iter(|| {
            r = black_box(expand_interval_jiff_iter(begin, end).unwrap());
        })
    });
    assert_eq!(r.len(), 366);
}

criterion_group!(benches, test_jiff, test_jiff_iter, test_jiff_series);
criterion_main!(benches);
