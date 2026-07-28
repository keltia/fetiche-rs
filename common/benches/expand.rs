//! Benchmark for expand_interval between chrono/jiff
//!
//! ```text
//! chrono+dateparser       time:   [4.9884 µs 5.0106 µs 5.0369 µs]
//!                         change: [−0.8854% −0.2825% +0.3640%] (p = 0.39 > 0.05)
//!                         No change in performance detected.
//!
//! jiff                    time:   [1.4378 µs 1.4401 µs 1.4427 µs]
//!                         change: [−0.4205% −0.1216% +0.1654%] (p = 0.41 > 0.05)
//!                         No change in performance detected.
//!
//! jiff_iter               time:   [3.5292 µs 3.5433 µs 3.5580 µs]
//!                         change: [−1.3919% −0.7678% −0.2062%] (p = 0.01 < 0.05)
//!                         Change within noise threshold.
//! ```

use std::hint::black_box;

use chrono::{Duration, Utc};
use criterion::{Criterion, criterion_group, criterion_main};
use jiff::Span;
use jiff::civil::DateTime;

pub fn expand_interval(
    begin: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
) -> eyre::Result<Vec<chrono::DateTime<Utc>>> {
    // Pre-calculate capacity
    let days = (end - begin).num_days();
    let mut intv = Vec::with_capacity(days as usize);

    let mut d = begin;
    while d < end {
        intv.push(d);
        d += Duration::days(1);
    }
    Ok(intv)
}

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
    // Pre-calculate capacity: days between begin and end
    let days_span = end.since(begin)?;
    let days_count = days_span.get_days();

    // Use successors to chain single-day additions (only creates one Span)
    let day = Span::new().days(1);
    let dates: Vec<_> = std::iter::successors(Some(begin), |&d| {
        let next = d.checked_add(day).ok()?;
        (next < end).then_some(next)
    })
        .collect();

    Ok(dates)
}

fn test_jiff(c: &mut Criterion) {
    let mut r = vec![];

    // 2024 has 366 days
    let begin = "2024-01-01".parse().unwrap();
    let end = "2025-01-01".parse().unwrap();

    c.bench_function("jiff", |b| {
        b.iter(|| {
            r = black_box(expand_interval_jiff(begin, end).unwrap());
        })
    });
    eprintln!("vec(jiff) = {}", r.len())
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
    eprintln!("vec(jiff_iter) = {}", r.len())
}

fn test_chrono(c: &mut Criterion) {
    let mut r = vec![];

    // 2024 has 366 days
    let begin = dateparser::parse("2024-01-01").unwrap();
    let end = dateparser::parse("2024-12-31").unwrap();

    c.bench_function("chrono+dateparser", |b| {
        b.iter(|| {
            r = black_box(expand_interval(begin, end).unwrap());
        })
    });
    eprintln!("vec(chrono) = {}", r.len())
}

criterion_group!(benches, test_chrono, test_jiff, test_jiff_iter);
criterion_main!(benches);
