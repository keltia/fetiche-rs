use std::fs;

use criterion::{criterion_group, criterion_main, Criterion};

use fetiche_formats::StateList;

/// Bench empty data
///
fn de_empty(c: &mut Criterion) {
    let data = r##"{"time": 1679999991, "states": null}"##;
    let mut sl: StateList = StateList {
        time: 0,
        states: None,
    };

    c.bench_function("empty statelist", |b| {
        b.iter(|| {
            sl = serde_json::from_str(data).unwrap();
        })
    });
    let _ = sl;
}

/// Bench a fake data packet
///
fn de_full(c: &mut Criterion) {
    let data = fs::read_to_string("../data/test.json").unwrap();

    let mut sl: StateList = StateList {
        time: 0,
        states: None,
    };

    c.bench_function("full statelist", |b| {
        b.iter(|| {
            sl = serde_json::from_str(&data).unwrap();
        })
    });
    let _ = sl;
}

criterion_group!(benches, de_empty, de_full);

criterion_main!(benches);
