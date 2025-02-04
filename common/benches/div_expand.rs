//! Using `Divan` instead of `Criterion`
//!
//! jiff DateTime/Timestamp vs chrono DateTime<Utc>
//!
//! AMD 7700X, 32 GB, rust 1.84.1
//! ```text
//! Timer precision: 100 ns
//! div_expand                         fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ test_chrono                     1.662 µs      │ 2.662 µs      │ 1.699 µs      │ 1.737 µs      │ 100     │ 800
//! ├─ test_jiff_duration              8.499 µs      │ 24.29 µs      │ 12.69 µs      │ 10.96 µs      │ 100     │ 100
//! ├─ test_jiff_duration_date         2.699 µs      │ 3.849 µs      │ 2.749 µs      │ 2.775 µs      │ 100     │ 400
//! ├─ test_jiff_duration_date_series  4.299 µs      │ 10.02 µs      │ 4.324 µs      │ 4.616 µs      │ 100     │ 400
//! ├─ test_jiff_duration_timestamp    1.874 µs      │ 7.512 µs      │ 2.799 µs      │ 3.159 µs      │ 100     │ 800
//! ├─ test_jiff_naive                 8.699 µs      │ 14.99 µs      │ 8.999 µs      │ 9.087 µs      │ 100     │ 100
//! ├─ test_jiff_timestamp_series      199.8 ns      │ 13.69 µs      │ 249.8 ns      │ 397.8 ns      │ 100     │ 100
//! ╰─ test_jiff_timestamp_series_h    2.349 µs      │ 3.212 µs      │ 2.462 µs      │ 2.681 µs      │ 100     │ 800
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
fn test_jiff_duration_date_series(bencher: Bencher) {
    use jiff::civil::Date;

    #[inline]
    fn expand_interval_jiff(begin: Date, end: Date) -> eyre::Result<Vec<Date>> {
        let intv = begin.series(1.days()).take_while(|&ts| ts < end).collect::<Vec<_>>();
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
        let intv = begin.series(1.days()).take_while(|&ts| ts < end).collect::<Vec<_>>();
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
        let intv = begin.series(24.hours()).take_while(|&ts| ts < end).collect::<Vec<_>>();
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
