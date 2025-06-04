//! Benchmark for part of the conversion from json to struct to dronepoint to csv.
//!
//! AMD Ryzen 7 7700X @5.4 GHz, Win 11H02
//! ```text
//! deserialize             time:   [2.8480 µs 2.8520 µs 2.8562 µs]
//! Found 8 outliers among 100 measurements (8.00%)
//!   6 (6.00%) high mild
//!   2 (2.00%) high severe
//!
//! convert                 time:   [127.31 ns 127.42 ns 127.55 ns]
//! Found 7 outliers among 100 measurements (7.00%)
//!   1 (1.00%) low mild
//!   3 (3.00%) high mild
//!   3 (3.00%) high severe
//!
//! prepare                 time:   [916.47 ns 917.25 ns 917.98 ns]
//! Found 1 outliers among 100 measurements (1.00%)
//!   1 (1.00%) high mild
//! ```
//!
//! Apple Silicon M1 Max @3.2 GHz, Sonoma 14.5
//! ```text
//! deserialize             time:   [3.2198 µs 3.2260 µs 3.2332 µs]
//!                         change: [−2.1302% −1.3497% −0.7747%] (p = 0.00 < 0.05)
//! Found 17 outliers among 100 measurements (17.00%)
//!   7 (7.00%) high mild
//!   10 (10.00%) high severe
//!
//! convert                 time:   [83.567 ns 83.988 ns 84.418 ns]
//!                         change: [−0.3280% +0.1356% +0.6018%] (p = 0.57 > 0.05)
//! Found 7 outliers among 100 measurements (7.00%)
//!   5 (5.00%) high mild
//!   2 (2.00%) high severe
//!
//! prepare                 time:   [937.01 ns 946.77 ns 958.18 ns]
//!                         change: [−0.7630% +0.6468% +2.0291%] (p = 0.38 > 0.05)
//! ```text
//!

use csv::{QuoteStyle, WriterBuilder};
use fetiche_formats::senhive::FusedData;
use fetiche_formats::DronePoint;
use serde::Serialize;
use std::fmt::Debug;
use std::hint::black_box;
use std::io::Cursor;

use criterion::{criterion_group, criterion_main, Criterion};

const DATA: &str = r##"
{"version":"1.0.0","system":{"trackID":"561c2855-d4a2-4109-aada-56ef79c00ffe","timestamp":"2024-10-23T13:02:03+00:00","timestampLog":[],"fusionState":{"fusionType":1,"sourceSerials":["1424823000354"]}},"vehicleIdentification":{"serial":"F5YHX23CR0030UT5","mac":null,"make":"DJI","model":"DJI Mini 3","uavType":2},"vehicleState":{"location":{"coordinates":{"lon":2.378793716430664,"lat":48.57561492919922},"uncertainty":null,"likelihood":"POLYGON ((2.3789294904782423 48.57577052279335, 2.379065263690095 48.57561492887997, 2.378929489642521 48.575459335445466, 2.3786579432188075 48.575459335445466, 2.3785221691712337 48.57561492887997, 2.3786579423830863 48.57577052279335, 2.3789294904782423 48.57577052279335))"},"altitudes":{"ato":{"value":106.5,"uncertainty":null},"agl":{"value":106.5,"uncertainty":null},"amsl":null,"geodetic":{"value":236.0,"uncertainty":null}},"groundSpeed":{"value":0.022360679774997897,"uncertainty":null},"verticalSpeed":null,"orientation":{"value":153.0,"uncertainty":null},"state":2},"pilotIdentification":null,"pilotState":{"location":{"coordinates":{"lon":2.378805160522461,"lat":48.57560348510742},"uncertainty":null,"likelihood":null},"locationType":2}}
"##;

pub fn prepare_csv<T>(data: T) -> eyre::Result<String>
where
    T: Serialize + Debug,
{
    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new()
        .has_headers(false)
        .quote_style(QuoteStyle::NonNumeric)
        .from_writer(vec![]);

    // Insert data
    //
    wtr.serialize(data)?;

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

fn do_deserialize(c: &mut Criterion) {
    c.bench_function("deserialize", move |b| {
        b.iter(|| {
            let cur = Cursor::new(DATA);
            let _: FusedData = black_box(serde_json::from_reader(cur).unwrap());
        })
    });
}

fn do_convert(c: &mut Criterion) {
    let cur = Cursor::new(DATA);
    let data: FusedData = serde_json::from_reader(cur).unwrap();

    c.bench_function("convert", move |b| {
        b.iter(|| {
            let _: DronePoint = black_box((&data).into());
        })
    });
}

fn do_prepare(c: &mut Criterion) {
    let cur = Cursor::new(DATA);
    let data: FusedData = serde_json::from_reader(cur).unwrap();
    let data: DronePoint = (&data).into();

    c.bench_function("prepare", move |b| {
        b.iter(|| {
            let data = data.clone();
            let _ = black_box(prepare_csv(data).unwrap());
        });
    });
}

criterion_group!(benches, do_deserialize, do_convert, do_prepare);

criterion_main!(benches);
