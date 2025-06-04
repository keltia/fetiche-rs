//! Benchmarks for the `json_to_nl` and `json_to_csv` subcommands.
//!
//! Mac Studio 2022
//! Apple Silicon M1 Max 3.2 GHz, Tahoe 26.0.1
//! ```text
//! Timer precision: 41 ns
//! json_to                    fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ bench_from_json_to_csv  249 ns        │ 44.08 µs      │ 291 ns        │ 996.5 ns      │ 100     │ 100
//! ╰─ bench_from_json_to_nl   794.7 µs      │ 1.836 ms      │ 815.7 µs      │ 841.4 µs      │ 100     │ 100
//! ```
//!
use divan::{black_box, Bencher};
use std::fs;
use std::io::Cursor;
use std::num::NonZeroUsize;

use csv::{QuoteStyle, WriterBuilder};
use polars::prelude::{JsonFormat, JsonReader, JsonWriter, SerReader, SerWriter};

use fetiche_formats::senhive::FusedData;
use fetiche_formats::DronePoint;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_from_json_to_nl(bench: Bencher) {
    let str = fs::read_to_string("../testdata/senhive-test.json").unwrap();
    bench.bench_local(move || black_box(from_json_to_nl(str.as_bytes())));
}

#[divan::bench]
fn bench_from_json_to_csv(bench: Bencher) {
    let str = fs::read_to_string("../testdata/senhive-test.json").unwrap();
    bench.bench_local(move || black_box(from_json_to_csv(str.as_bytes())));
}

// See subr.rs for the actual implementation.

fn from_json_to_nl(data: &[u8]) -> eyre::Result<String> {
    let cur = Cursor::new(data);
    let mut df = JsonReader::new(cur)
        .with_json_format(JsonFormat::Json)
        .infer_schema_len(NonZeroUsize::new(3))
        .finish()?;

    let mut buf = vec![];
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::JsonLines)
        .finish(&mut df)?;
    Ok(String::from_utf8(buf)?)
}

fn from_json_to_csv(data: &[u8]) -> eyre::Result<String> {
    let cur = Cursor::new(data);
    let data: FusedData = serde_json::from_reader(cur)?;
    let data: DronePoint = (&data).into();

    let mut wtr = WriterBuilder::new()
        .has_headers(false)
        .quote_style(QuoteStyle::NonNumeric)
        .from_writer(vec![]);

    // Insert data
    //
    wtr.serialize(data)?;

    // Output final csv line
    //
    let data = String::from_utf8(wtr.into_inner()?)?;

    Ok(data)
}

