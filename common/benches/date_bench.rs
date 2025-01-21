use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jiff::civil::DateTime;

fn test_humantime(c: &mut Criterion) {
    let base = "2024-03-08 12:34:56";
    let mut curr = Utc::now();

    c.bench_function("humantime", |b| {
        b.iter(|| {
            let this = black_box(humantime::parse_rfc3339_weak(base).unwrap());
            curr = this.into();
        })
    });
    let _ = curr;
}

fn test_jiff(c: &mut Criterion) {
    let base = "2024-03-08 12:34:56";
    let mut curr: DateTime = DateTime::ZERO;

    c.bench_function("jiff", |b| {
        b.iter(|| {
            curr = base.parse().unwrap();
        })
    });
}

fn test_dateparser(c: &mut Criterion) {
    let base = "2024-03-08 12:34:56";
    let mut curr = Utc::now();

    c.bench_function("dateparser", |b| {
        b.iter(|| {
            curr = black_box(dateparser::parse(base).unwrap());
        })
    });
    let _ = curr;
}

criterion_group!(benches, test_dateparser, test_humantime, test_jiff);
criterion_main!(benches);
