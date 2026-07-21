//! Benchmark comparing serde_json vs rkyv for CubeData
//!
//! CubeData is the high-volume streaming format from Avionix CUBE antenna
//! with 30+ fields representing ADS-B/FLARM/Remote-ID data.
//!
use criterion::{criterion_group, criterion_main, Criterion};
use fetiche_formats::avionix::{CubeData, RCubeData};

use std::hint::black_box;

const SAMPLE_JSON: &str = r#"{
    "uti": 1696123456,
    "dat": "2024-02-24T12:34:56.789123456",
    "hex": "ABCDEF",
    "tim": "12:34:56.789",
    "fli": "TEST123",
    "lat": 51.5074,
    "lon": -0.1278,
    "gda": "A",
    "src": "A",
    "alt": 10000,
    "altg": 9500,
    "hgt": 500,
    "spd": 450,
    "cat": "A2",
    "squ": "7700",
    "vrt": -1200,
    "trk": 270,
    "mop": 2,
    "lla": 1,
    "tru": 543,
    "dbm": -85,
    "shd": 270,
    "org": "KJFK",
    "dst": "KLAX",
    "opr": "AAL",
    "typ": "B738",
    "reg": "N12345",
    "cou": "USA"
}"#;

fn benchmark_cube_serde(c: &mut Criterion) {
    let cube: CubeData = serde_json::from_str(SAMPLE_JSON).unwrap();
    let json = serde_json::to_string(&cube).unwrap();

    c.bench_function("cube_json_serialize", |b| {
        b.iter(|| {
            let _ = black_box(serde_json::to_string(&cube).unwrap());
        });
    });

    c.bench_function("cube_json_deserialize", |b| {
        b.iter(|| {
            let _: CubeData = black_box(serde_json::from_str(&json).unwrap());
        });
    });
}

fn benchmark_cube_rkyv(c: &mut Criterion) {
    let cube: CubeData = serde_json::from_str(SAMPLE_JSON).unwrap();
    let rkyv_cube: RCubeData = (&cube).into();
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&rkyv_cube).unwrap();

    c.bench_function("cube_rkyv_convert", |b| {
        b.iter(|| {
            let _: RCubeData = black_box((&cube).into());
        });
    });

    c.bench_function("cube_rkyv_serialize", |b| {
        b.iter(|| {
            let _ = black_box(rkyv::to_bytes::<rkyv::rancor::Error>(&rkyv_cube).unwrap());
        });
    });

    c.bench_function("cube_rkyv_deserialize", |b| {
        b.iter(|| {
            let _: RCubeData =
                black_box(rkyv::from_bytes::<RCubeData, rkyv::rancor::Error>(&bytes).unwrap());
        });
    });

    c.bench_function("cube_rkyv_full_cycle", |b| {
        b.iter(|| {
            // Realistic use case: convert, serialize, deserialize
            let rkyv_cube: RCubeData = black_box((&cube).into());
            let bytes = black_box(rkyv::to_bytes::<rkyv::rancor::Error>(&rkyv_cube).unwrap());
            let _: RCubeData =
                black_box(rkyv::from_bytes::<RCubeData, rkyv::rancor::Error>(&bytes).unwrap());
        });
    });
}

criterion_group!(benches, benchmark_cube_serde, benchmark_cube_rkyv);
criterion_main!(benches);
