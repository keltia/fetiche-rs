//! Using `Divan` instead of `Criterion`
//!
//! jiff DateTime/Timestamp vs chrono DateTime<Utc>
//!
//! AMD 7700X, 32 GB, rust 1.84.1
//! ```text
//! Timer precision: 100 ns
//! div_expand                       fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ test_chrono                   2.167 µs      │ 3.599 µs      │ 2.482 µs      │ 2.617 µs      │ 100     │ 100000
//! ├─ test_jiff_duration            17 µs         │ 23.18 µs      │ 17.8 µs       │ 18.04 µs      │ 100     │ 100000
//! ├─ test_jiff_duration_date       11.2 µs       │ 13.52 µs      │ 11.74 µs      │ 11.78 µs      │ 100     │ 100000
//! ├─ test_jiff_duration_timestamp  2.807 µs      │ 4.302 µs      │ 3.189 µs      │ 3.229 µs      │ 100     │ 100000
//! ╰─ test_jiff_naive               39.28 µs      │ 58.69 µs      │ 40.73 µs      │ 42.36 µs      │ 100     │ 100000
//! ```
//! Thanks to Bsky: @burntsushi.net (Andrew Gallant)
//!

fn main() {
    divan::main();
}

use divan::{black_box, Bencher};

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
