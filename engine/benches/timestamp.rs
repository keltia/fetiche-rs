use divan::{black_box, Bencher};
use jiff::civil::DateTime;
use jiff::Timestamp;

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