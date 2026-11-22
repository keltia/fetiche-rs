//! Micro-benchmark for strftime/format between chrono and jiff
//!
//! PC, AMD 7700X, 32 GB, 1 TB M2 SSD, Win11 25H2, Rust 1.100.0, jiff 1.0.2
//! ```text
//! Timer precision: 100 ns
//! format                 fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ test_jiff_format    113.8 ns      │ 146.6 ns      │ 114.6 ns      │ 115.2 ns      │ 100     │ 12800
//! ╰─ test_jiff_strftime  99.82 ns      │ 17.79 µs      │ 99.82 ns      │ 277.8 ns      │ 100     │ 100
//! ```
//!

use std::hint::black_box;

use divan::Bencher;

fn main() {
    divan::main();
}

#[divan::bench]
fn test_jiff_format(bencher: Bencher) {
    use jiff::ZonedDateTime;

    fn format_day(day: &ZonedDateTime) -> String {
        format!("{:04}-{:02}-{:02}", day.year(), day.month(), day.day()).to_string()
    }

    let day = ZonedDateTime::now();
    bencher.bench_local(|| {
        black_box(format_day(&day));
    });
}

#[divan::bench]
fn test_jiff_strftime(bencher: Bencher) {
    use jiff::ZonedDateTime;

    fn strftime_day(day: &ZonedDateTime) -> String {
        day.strftime("%Y-%m-%d").to_string()
    }

    let day = ZonedDateTime::now();
    bencher.bench_local(|| {
        black_box(strftime_day(&day));
    });
}
