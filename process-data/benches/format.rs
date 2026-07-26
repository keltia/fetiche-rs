//! Micro-benchmark for strftime/format between chrono and jiff
//!
//! PC, AMD 7700X, 32 GB, 1 TB M2 SSD, Win11 25H2
//! ```text
//! Timer precision: 100 ns
//! format                   fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ test_chrono_format    128.7 ns      │ 163 ns        │ 130.2 ns      │ 131.4 ns      │ 100     │ 12800
//! ├─ test_chrono_strftime  218.5 ns      │ 315.4 ns      │ 231 ns        │ 236.6 ns      │ 100     │ 6400
//! ├─ test_jiff_format      120.1 ns      │ 252.1 ns      │ 127.1 ns      │ 128.6 ns      │ 100     │ 12800
//! ╰─ test_jiff_strftime    85.74 ns      │ 88.87 ns      │ 86.13 ns      │ 86.27 ns      │ 100     │ 12800
//! ```
//!

use std::hint::black_box;

use divan::Bencher;

fn main() {
    divan::main();
}

#[divan::bench]
fn test_chrono_format(bencher: Bencher) {
    use chrono::{DateTime, Datelike, Utc};

    fn format_day(day: &DateTime<Utc>) -> String {
        format!("{:04}-{:02}-{:02}", day.year(), day.month(), day.day()).to_string()
    }

    let day = Utc::now();
    bencher.bench_local(|| {
        black_box(format_day(&day));
    });
}

#[divan::bench]
fn test_chrono_strftime(bencher: Bencher) {
    use chrono::{DateTime, Utc};

    fn strftime_day(day: &DateTime<Utc>) -> String {
        day.format("%Y-%m-%d").to_string()
    }

    let day = Utc::now();
    bencher.bench_local(|| {
        black_box(strftime_day(&day));
    });
}

#[divan::bench]
fn test_jiff_format(bencher: Bencher) {
    use jiff::Zoned;

    fn format_day(day: &Zoned) -> String {
        format!("{:04}-{:02}-{:02}", day.year(), day.month(), day.day()).to_string()
    }

    let day = Zoned::now();
    bencher.bench_local(|| {
        black_box(format_day(&day));
    });
}

#[divan::bench]
fn test_jiff_strftime(bencher: Bencher) {
    use jiff::Zoned;

    fn strftime_day(day: &Zoned) -> String {
        day.strftime("%Y-%m-%d").to_string()
    }

    let day = Zoned::now();
    bencher.bench_local(|| {
        black_box(strftime_day(&day));
    });
}
