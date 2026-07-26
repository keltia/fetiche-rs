use std::hint::black_box;

use divan::Bencher;
use jiff::Zoned;

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
    use chrono::{DateTime, Datelike, Utc};

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
    fn strftime_day(day: &Zoned) -> String {
        day.strftime("%Y-%m-%d").to_string()
    }

    let day = Zoned::now();
    bencher.bench_local(|| {
        black_box(strftime_day(&day));
    });
}
