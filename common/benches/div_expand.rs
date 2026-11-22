//! jiff DateTime/Timestamp
//!
//! AMD 7700X, 32 GB, rust 1.100.0
//! ```text
//! Timer precision: 100 ns
//! div_expand                         fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ test_jiff_duration              7.849 µs      │ 10.04 µs      │ 7.949 µs      │ 8.008 µs      │ 100     │ 200
//! ├─ test_jiff_duration_date         1.699 µs      │ 2.499 µs      │ 1.724 µs      │ 1.739 µs      │ 100     │ 800
//! ├─ test_jiff_duration_date_series  3.874 µs      │ 4.949 µs      │ 3.924 µs      │ 3.939 µs      │ 100     │ 400
//! ├─ test_jiff_duration_timestamp    874.8 ns      │ 5.199 µs      │ 874.8 ns      │ 940.5 ns      │ 100     │ 400
//! ├─ test_jiff_naive                 4.799 µs      │ 5.249 µs      │ 4.849 µs      │ 4.865 µs      │ 100     │ 200
//! ├─ test_jiff_timestamp_series      99.81 ns      │ 17.09 µs      │ 99.81 ns      │ 287.8 ns      │ 100     │ 100
//! ╰─ test_jiff_timestamp_series_h    1.262 µs      │ 5.362 µs      │ 1.287 µs      │ 1.348 µs      │ 100     │ 800
//! ```
//! Thanks to Bsky: @burntsushi.net (Andrew Gallant)
//!

fn main() {
    divan::main();
}

use divan::{Bencher, black_box};
use jiff::{Timestamp, ToSpan};

#[divan::bench]
fn test_jiff_naive(bencher: Bencher) {
    use jiff::Span;
    use jiff::civil::DateTime;

    #[inline]
    fn expand_interval_jiff(begin: DateTime, end: DateTime) -> eyre::Result<Vec<DateTime>> {
        let mut d = begin;
        let mut intv = vec![];

        let day = Span::new().days(1);
        while d < end {
            intv.push(d);
            d = d.checked_add(day).expect("overflow");
        }
        Ok(intv)
    }

    let begin = "2024-01-01".parse().unwrap();
    let end = "2024-12-31".parse().unwrap();
    bencher.bench_local(move || black_box(expand_interval_jiff(begin, end)));
}

#[divan::bench]
fn test_jiff_duration(bencher: Bencher) {
    use jiff::SignedDuration;
    use jiff::civil::DateTime;

    #[inline]
    fn expand_interval_jiff(begin: DateTime, end: DateTime) -> eyre::Result<Vec<DateTime>> {
        let mut d = begin;
        let mut intv = vec![];

        let day = SignedDuration::from_hours(24);
        while d < end {
            intv.push(d);
            d = d.checked_add(day).expect("overflow");
        }
        Ok(intv)
    }

    let begin = "2024-01-01".parse().unwrap();
    let end = "2024-12-31".parse().unwrap();
    bencher.bench_local(move || black_box(expand_interval_jiff(begin, end)));
}

#[divan::bench]
fn test_jiff_duration_date(bencher: Bencher) {
    use jiff::SignedDuration;
    use jiff::civil::Date;

    #[inline]
    fn expand_interval_jiff(begin: Date, end: Date) -> eyre::Result<Vec<Date>> {
        let mut d = begin;
        let mut intv = vec![];

        let day = SignedDuration::from_hours(24);
        while d < end {
            intv.push(d);
            d = d.checked_add(day).expect("overflow");
        }
        Ok(intv)
    }

    let begin: Date = "2024-01-01".parse().unwrap();
    let end: Date = "2024-12-31".parse().unwrap();
    bencher.bench_local(move || black_box(expand_interval_jiff(begin, end)));
}

#[divan::bench]
fn test_jiff_duration_date_series(bencher: Bencher) {
    use jiff::civil::Date;

    #[inline]
    fn expand_interval_jiff(begin: Date, end: Date) -> eyre::Result<Vec<Date>> {
        let intv = begin
            .series(1.days())
            .take_while(|&ts| ts < end)
            .collect::<Vec<_>>();
        Ok(intv)
    }

    let begin: Date = "2024-01-01".parse().unwrap();
    let end: Date = "2024-12-31".parse().unwrap();
    bencher.bench_local(move || black_box(expand_interval_jiff(begin, end)));
}

#[divan::bench]
fn test_jiff_timestamp_series(bencher: Bencher) {
    #[inline]
    fn expand_interval_jiff(begin: Timestamp, end: Timestamp) -> eyre::Result<Vec<Timestamp>> {
        let intv = begin
            .series(24.hours())
            .take_while(|&ts| ts < end)
            .collect::<Vec<_>>();
        Ok(intv)
    }

    let begin: Timestamp = "2024-01-01T00:00Z".parse().unwrap();
    let end: Timestamp = "2024-12-31T00:00Z".parse().unwrap();
    bencher.bench_local(move || black_box(expand_interval_jiff(begin, end)));
}

#[divan::bench]
fn test_jiff_timestamp_series_h(bencher: Bencher) {
    #[inline]
    fn expand_interval_jiff(begin: Timestamp, end: Timestamp) -> eyre::Result<Vec<Timestamp>> {
        let intv = begin
            .series(24.hours())
            .take_while(|&ts| ts < end)
            .collect::<Vec<_>>();
        Ok(intv)
    }

    let begin: Timestamp = "2024-01-01T00:00Z".parse().unwrap();
    let end: Timestamp = "2024-12-31T00:00Z".parse().unwrap();
    bencher.bench_local(move || black_box(expand_interval_jiff(begin, end)));
}

#[divan::bench]
fn test_jiff_duration_timestamp(bencher: Bencher) {
    use jiff::{SignedDuration, Timestamp};

    #[inline]
    fn expand_interval_jiff(begin: Timestamp, end: Timestamp) -> eyre::Result<Vec<Timestamp>> {
        let mut d = begin;
        let mut intv = vec![];

        let day = SignedDuration::from_hours(24);
        while d < end {
            intv.push(d);
            d = d.checked_add(day).expect("overflow");
        }
        Ok(intv)
    }

    let begin: Timestamp = "2024-01-01T00:00Z".parse().unwrap();
    let end: Timestamp = "2024-12-31T00:00Z".parse().unwrap();
    bencher.bench_local(move || black_box(expand_interval_jiff(begin, end)));
}
