//! Example on how to use a function to transform a specific column in a Dataframe with polars.
//!
//! Benchmarking dateparser (which we know as very slow) versus jiff.
//!
//! Mac Studio 2022, 64 GB, 4 TB, macOS 26.5
//! ```text
//! Gnuplot not found, using plotters backend
//! dateparser              time:   [4.9951 µs 5.0242 µs 5.0537 µs]
//!                         change: [−0.2916% +0.3098% +0.9034%] (p = 0.31 > 0.05)
//!
//! jiff_zoned              time:   [309.03 ns 309.68 ns 310.38 ns]
//!                         change: [+0.2035% +0.4903% +0.7676%] (p = 0.00 < 0.05)
//!
//! jiff_datetime           time:   [326.47 ns 327.18 ns 327.89 ns]
//!                         change: [−0.6159% −0.2920% −0.0046%] (p = 0.06 > 0.05)
//!
//! humantime               time:   [230.27 ns 231.00 ns 231.79 ns]
//!                         change: [+0.2193% +0.5983% +0.9186%] (p = 0.00 < 0.05)
//! ```

use std::hint::black_box;
use std::io::Cursor;
use std::time::UNIX_EPOCH;

use criterion::{Criterion, criterion_group, criterion_main};
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use polars::datatypes::Int64Chunked;
use polars::prelude::{Column, CsvParseOptions, CsvReadOptions, IntoColumn, SerReader};

fn into_timestamp(col: &Column) -> Column {
    col.str()
        .unwrap()
        .iter()
        .map(|d: Option<&str>| d.map(|d: &str| dateparser::parse(d).unwrap().timestamp()))
        .collect::<Int64Chunked>()
        .into_column()
}

fn into_humantime_secs(col: &Column) -> Column {
    col.str()
        .unwrap()
        .iter()
        .map(|d: Option<&str>| {
            d.map(|d: &str| {
                humantime::parse_rfc3339_weak(d)
                    .unwrap()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            })
        })
        .collect::<Int64Chunked>()
        .into_column()
}

fn into_zoned_jiff(col: &Column) -> Column {
    col.str()
        .unwrap()
        .iter()
        .map(|d: Option<&str>| {
            d.map(|d: &str| {
                d.parse::<DateTime>()
                    .unwrap()
                    .to_zoned(TimeZone::UTC)
                    .unwrap()
                    .timestamp()
                    .as_second()
            })
        })
        .collect::<Int64Chunked>()
        .into_column()
}

fn into_timestamp_jiff(col: &Column) -> Column {
    col.str()
        .unwrap()
        .iter()
        .map(|d: Option<&str>| {
            d.map(|d: &str| {
                d.parse::<DateTime>()
                    .unwrap()
                    .duration_since(DateTime::MIN)
                    .as_secs()
            })
        })
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
        .finish()
        .unwrap();
    df.column("timestamp").unwrap().clone()
}

fn dateparser(c: &mut Criterion) {
    let vl = setup();
    c.bench_function("dateparser", |b| {
        b.iter({
            let v = vl.clone();
            move || {
                black_box(into_timestamp(&v));
            }
        })
    });
}

fn jiff_datetime(c: &mut Criterion) {
    let vl = setup();
    c.bench_function("jiff_datetime", |b| {
        b.iter({
            let v = vl.clone();
            move || {
                black_box(into_timestamp_jiff(&v));
            }
        })
    });
}

fn jiff_zoned(c: &mut Criterion) {
    let vl = setup();
    c.bench_function("jiff_zoned", |b| {
        b.iter({
            let v = vl.clone();
            move || {
                black_box(into_zoned_jiff(&v));
            }
        })
    });
}

fn humantime(c: &mut Criterion) {
    let vl = setup();
    c.bench_function("humantime", |b| {
        b.iter({
            let v = vl.clone();
            move || {
                black_box(into_humantime_secs(&v));
            }
        })
    });
}

criterion_group!(benches, dateparser, jiff_zoned, jiff_datetime, humantime);
criterion_main!(benches);
