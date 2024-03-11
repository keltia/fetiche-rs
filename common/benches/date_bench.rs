use chrono::{DateTime, Utc};
use criterion::{black_box, Criterion, criterion_group, criterion_main};

fn test_humantime(c: &mut Criterion) {
    let base = "2024-03-08 12:34:56";
    let mut curr = Utc::now();

    c.bench_function("humantime", |b| {
        b.iter(|| {
            let this = black_box(humantime::parse_rfc3339_weak(base).unwrap());
            curr = curr.into();
        })
    });
    let _ = curr;
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

criterion_group!(benches, test_dateparser, test_humantime);
criterion_main!(benches);


