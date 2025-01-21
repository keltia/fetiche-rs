use chrono::{Duration, Utc};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jiff::civil::DateTime;
use jiff::Span;

pub fn expand_interval(
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

pub fn expand_interval_jiff(begin: DateTime, end: DateTime) -> eyre::Result<Vec<DateTime>> {
    let mut d = begin;
    let mut intv = vec![];

    let day = Span::new().days(1);
    while d < end {
        intv.push(d);
        d = d.checked_add(day).expect("overflow");
    }
    Ok(intv)
}

fn test_jiff(c: &mut Criterion) {
    let begin = "2024-01-01".parse().unwrap();
    let end = "2024-12-31".parse().unwrap();

    c.bench_function("jiff", |b| {
        b.iter(|| {
            let _ = black_box(expand_interval_jiff(begin, end).unwrap());
        })
    });
}

fn test_chrono(c: &mut Criterion) {
    let begin = dateparser::parse("2024-01-01").unwrap();
    let end = dateparser::parse("2024-12-31").unwrap();

    c.bench_function("chrono+dateparser", |b| {
        b.iter(|| {
            let _ = black_box(expand_interval(begin, end).unwrap());
        })
    });
}

criterion_group!(benches, test_chrono, test_jiff);
criterion_main!(benches);
