fn main() {
    divan::main();
}

use divan::{black_box, Bencher};

#[divan::bench]
fn test_jiff(bencher: Bencher) {
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
