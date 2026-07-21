//! Benchmark comparing serde_json vs rkyv for AlertData
//!
use chrono::DateTime;
use criterion::{criterion_group, criterion_main, Criterion};
use fetiche_formats::senhive::{AlertData, RAlertData, Severity};

use std::hint::black_box;

fn benchmark_alert_serde(c: &mut Criterion) {
    let alert = AlertData {
        version: Some("1.0.0".to_string()),
        title: "Critical system failure detected".to_string(),
        timestamp: DateTime::from_timestamp(1234567890, 0).unwrap(),
        severity: Severity::Critical,
        details: "Database connection pool exhausted. Immediate attention required.".to_string(),
    };

    let json = serde_json::to_string(&alert).unwrap();

    c.bench_function("alert_json_serialize", |b| {
        b.iter(|| {
            let _ = black_box(serde_json::to_string(&alert).unwrap());
        });
    });

    c.bench_function("alert_json_deserialize", |b| {
        b.iter(|| {
            let _: AlertData = black_box(serde_json::from_str(&json).unwrap());
        });
    });
}

fn benchmark_alert_rkyv(c: &mut Criterion) {
    let alert = AlertData {
        version: Some("1.0.0".to_string()),
        title: "Critical system failure detected".to_string(),
        timestamp: DateTime::from_timestamp(1234567890, 0).unwrap(),
        severity: Severity::Critical,
        details: "Database connection pool exhausted. Immediate attention required.".to_string(),
    };

    let rkyv_alert: RAlertData = (&alert).into();
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&rkyv_alert).unwrap();

    c.bench_function("alert_rkyv_serialize", |b| {
        b.iter(|| {
            let _ = black_box(rkyv::to_bytes::<rkyv::rancor::Error>(&rkyv_alert).unwrap());
        });
    });

    c.bench_function("alert_rkyv_deserialize", |b| {
        b.iter(|| {
            let _: RAlertData =
                black_box(rkyv::from_bytes::<RAlertData, rkyv::rancor::Error>(&bytes).unwrap());
        });
    });
}

criterion_group!(benches, benchmark_alert_serde, benchmark_alert_rkyv);
criterion_main!(benches);
