//! Using `Divan` instead of `Criterion`
//!
//! jiff DateTime/Timestamp vs chrono DateTime<Utc>
//!
//! AMD 7700X, 32 GB, rust 1.84.1
//! ```text
//! Timer precision: 100 ns
//! div_expand                       fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ test_chrono                   2.161 µs      │ 3.641 µs      │ 2.405 µs      │ 2.492 µs      │ 100     │ 100000
//! ├─ test_jiff_duration            16.65 µs      │ 20.05 µs      │ 17.34 µs      │ 17.42 µs      │ 100     │ 100000
//! ├─ test_jiff_duration_date       10.85 µs      │ 12.09 µs      │ 11.29 µs      │ 11.3 µs       │ 100     │ 100000
//! ├─ test_jiff_duration_timestamp  2.455 µs      │ 3.754 µs      │ 2.903 µs      │ 2.986 µs      │ 100     │ 100000
//! ├─ test_jiff_naive               37.65 µs      │ 45.46 µs      │ 39.21 µs      │ 39.3 µs       │ 100     │ 100000
//! ╰─ test_jiff_timestamp_series    132.7 ns      │ 315.5 ns      │ 139 ns        │ 149.7 ns      │ 100     │ 100000
//! ```
//! Thanks to Bsky: @burntsushi.net (Andrew Gallant)
//!

fn main() {
    divan::main();
}

use divan::{black_box, Bencher};
use jiff::{Timestamp, ToSpan};

#[divan::bench]
fn test_jiff_naive(bencher: Bencher) {
    use jiff::civil::DateTime;
    use jiff::Span;

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
    use jiff::civil::DateTime;
    use jiff::SignedDuration;

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
    use jiff::civil::Date;
    use jiff::SignedDuration;

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
fn test_jiff_timestamp_series(bencher: Bencher) {
    use jiff::civil::Date;
    use jiff::SignedDuration;

    #[inline]
    fn expand_interval_jiff(begin: Timestamp, end: Timestamp) -> eyre::Result<Vec<Timestamp>> {
        let intv = begin.series(1.days()).take_while(|&ts| ts <= end).collect::<Vec<_>>();
        Ok(intv)
    }

    let begin: Timestamp = "2024-01-01 00:00:00-00".parse().unwrap();
    let end: Timestamp = "2024-12-31 00:00:00-00".parse().unwrap();
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

#[divan::bench]
fn test_chrono(bencher: Bencher) {
    use chrono::{Duration, Utc};

    #[inline]
    fn expand_interval(
        begin: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> eyre::Result<Vec<chrono::DateTime<Utc>>> {
        let mut d = begin;
        let mut intv = vec![];

        while d < end {
            intv.push(d);
            d += Duration::days(1);
        }
        Ok(intv)
    }

    let begin = dateparser::parse("2024-01-01").unwrap();
    let end = dateparser::parse("2024-12-31").unwrap();
    bencher.bench_local(move || black_box(expand_interval(begin, end)));
}
