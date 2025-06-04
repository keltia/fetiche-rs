//! timestamp generation benchmark.
//!
//! Timer precision: 41 ns
//! timestamp            fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ bench_dateparser  82.67 ns      │ 107.3 µs      │ 83.67 ns      │ 1.177 µs      │ 100     │ 100
//! ╰─ bench_jiff        2.29 µs       │ 7.249 µs      │ 2.374 µs      │ 2.455 µs      │ 100     │ 100
//!
use divan::{black_box, Bencher};

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_dateparser(bench: Bencher) {
    #[inline]
    fn dateparser_timestamp(date: &str) -> i64 {
        let date = dateparser::parse(&date).unwrap();
        date.timestamp()
    }

    bench.bench_local(move || black_box(dateparser_timestamp("2023-08-02T00:00:00Z")));
}

#[divan::bench]
fn bench_jiff(bench: Bencher) {
    use jiff::Timestamp;

    #[inline]
    fn jiff_timestamp(date: &str) -> i64 {
        let date: Timestamp = date.parse().unwrap();
        date.as_second()
    }

    bench.bench_local(move || black_box(jiff_timestamp("2023-08-02T00:00:00Z")));
}