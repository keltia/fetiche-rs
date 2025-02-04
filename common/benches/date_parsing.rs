//! Example on how to use a function to transform a specific column in a Dataframe with polars.
//!
//! Benchmarking dateparser (which we know as very slow) versus jiff.
//!
//! ```text
//! dateparser              time:   [5.4219 µs 5.4536 µs 5.4864 µs]
//!                         change: [-3.3857% -2.9426% -2.4739%] (p = 0.00 < 0.05)
//!                         Performance has improved.
//!
//! jiff                    time:   [695.33 ns 697.27 ns 699.25 ns]
//!                         change: [-1.7639% -1.3243% -0.8776%] (p = 0.00 < 0.05)
//!                         Change within noise threshold.
//! Found 6 outliers among 100 measurements (6.00%)
//!   6 (6.00%) low mild
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use polars::datatypes::Int64Chunked;
use polars::prelude::{Column, CsvParseOptions, CsvReadOptions, IntoColumn, SerReader};
use std::io::Cursor;
use std::time::UNIX_EPOCH;

fn into_timestamp(col: &Column) -> Column {
    col.str()
        .unwrap()
        .into_iter()
        .map(|d: Option<&str>| d.map(|d: &str| dateparser::parse(d).unwrap().timestamp()))
        .collect::<Int64Chunked>()
        .into_column()
}

fn into_humantime_secs(col: &Column) -> Column {
    col.str()
        .unwrap()
        .into_iter()
        .map(|d: Option<&str>| d.map(|d: &str| humantime::parse_rfc3339_weak(d).unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64))
        .collect::<Int64Chunked>()
        .into_column()
}

fn into_timestamp_jiff(col: &Column) -> Column {
    col.str()
        .unwrap()
        .into_iter()
        .map(|d: Option<&str>| d.map(|d: &str| {
            d.parse::<DateTime>().unwrap().to_zoned(TimeZone::UTC).unwrap().timestamp().as_second()
        }))
        .collect::<Int64Chunked>()
        .into_column()
}

fn setup() -> Column {
    let data = r##"journey,ident,model,source,location,timestamp,latitude,longitude,altitude,elevation,gps,rssi,home_lat,home_lon,home_height,speed,heading,station_name,station_latitude,station_longitude
72709,F6Z9C242V003PQBK,"DJI Mini4 Pro",as,2527943,"2024-12-09 11:04:59",34.710918,32.571717,108,92,,,34.711101,32.571637,16,0,338,0QRDKC2R03J32P,34.718506,32.475510
72706,L2T0023RB7,"Mini 2 SE",as,2527854,"2024-12-09 06:11:48",48.156054,16.350434,312,201,,,48.155487,16.350984,113,0,343,0QRDKC2R038370,48.104234,16.589570
72706,L2T0023RB7,"Mini 2 SE",as,2527855,"2024-12-09 06:11:58",48.156054,16.350434,312,201,,,48.155487,16.350984,113,0,343,0QRDKC2R038370,48.104234,16.589570
72706,L2T0023RB7,"Mini 2 SE",as,2527856,"2024-12-09 06:11:59",48.156054,16.350434,312,201,,,48.155487,16.350984,113,0,343,0QRDKC2R038370,48.104234,16.589570
72706,L2T1023RB7,"Mini 2 SE",as,2527856,"2024-12-09 06:12:59",48.156054,16.350434,312,201,,,48.155487,16.350984,113,0,343,0QRDKC2R038370,48.104234,16.589570
"##;

    // We need to fix the timestamp field.
    //
    let cur = Cursor::new(&data);
    let opts = CsvParseOptions::default().with_try_parse_dates(false);
    let df = CsvReadOptions::default()
        .with_has_header(true)
        .with_parse_options(opts)
        .into_reader_with_file_handle(cur)
        .finish().unwrap();
    df.column("timestamp").unwrap().clone()
}

fn dateparser(c: &mut Criterion) {
    let vl = setup();
    c.bench_function("dateparser", |b| b.iter({
        let v = vl.clone();
        move || {
            black_box(into_timestamp(&v));
        }
    }));
}

fn jiff(c: &mut Criterion) {
    let vl = setup();
    c.bench_function("jiff", |b| b.iter({
        let v = vl.clone();
        move || {
            black_box(into_timestamp_jiff(&v));
        }
    }));
}

fn humantime(c: &mut Criterion) {
    let vl = setup();
    c.bench_function("humantime", |b| b.iter({
        let v = vl.clone();
        move || {
            black_box(into_humantime_secs(&v));
        }
    }));
}

criterion_group!(benches, dateparser, jiff, humantime);
criterion_main!(benches);
