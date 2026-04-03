//! Benchmarks for finding airports by country.
//!
//! We check whether the expression makes any difference in searching (TL;DR: not really)
//!
//! ``text
//! Timer precision: 100 ns
//! bycountry              fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ find_with_contains  1.961 ms      │ 13.15 ms      │ 2.132 ms      │ 2.269 ms      │ 100     │ 100
//! ╰─ find_with_eq        1.906 ms      │ 2.727 ms      │ 2.094 ms      │ 2.11 ms       │ 100     │ 100
//! ```
//!

use std::hint::black_box;

use divan::Bencher;
use polars::prelude::{col, lit, LazyFrame};

fn main() {
    divan::main();
}

#[divan::bench]
fn find_with_contains(bencher: Bencher) {
    let name = "FR";
    let fname = "../data/airports.parquet";

    let expr = col("iso_country").str().contains(lit(name), true);
    bencher.bench_local(move || {
        let expr = expr.clone();
        let _ = black_box(
            LazyFrame::scan_parquet(fname.into(), Default::default())
                .unwrap()
                .select([
                    col("ident"),
                    col("name"),
                    col("latitude_deg"),
                    col("longitude_deg"),
                    col("elevation_ft"),
                    col("iata_code"),
                    col("iso_country"),
                ])
                .filter(expr)
                .collect()
                .unwrap(),
        );
    })
}

#[divan::bench]
fn find_with_eq(bencher: Bencher) {
    let name = "FR";
    let fname = "../data/airports.parquet";

    let expr = col("iso_country").eq(lit(name));
    bencher.bench_local(move || {
        let expr = expr.clone();
        let _ = black_box(
            LazyFrame::scan_parquet(fname.into(), Default::default())
                .unwrap()
                .select([
                    col("ident"),
                    col("name"),
                    col("latitude_deg"),
                    col("longitude_deg"),
                    col("elevation_ft"),
                    col("iata_code"),
                    col("iso_country"),
                ])
                .filter(expr)
                .collect()
                .unwrap(),
        );
    })
}
