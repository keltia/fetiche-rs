//! Benchmarking deserialization performance
//!
//! Original was using izip!
//! Improved is now using rayon.
//!
//! ```text
//! Timer precision: 100 ns
//! deserialize     fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ bench_izip   1.523 s       │ 1.893 s       │ 1.594 s       │ 1.615 s       │ 100     │ 100
//! ╰─ bench_rayon  497 ms        │ 807 ms        │ 670 ms        │ 671.8 ms      │ 100     │ 100
//! ```
//!

use std::hint::black_box;

use divan::Bencher;
use fetiche_common::find_tz;
use itertools::izip;
use pluscodes::Coordinate;
use polars::prelude::*;
use rayon::prelude::*;
use serde::Deserialize;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_izip(bencher: Bencher) {
    let lf = generate_df().unwrap();
    bencher.bench_local(move || {
        let lf = lf.clone();
        let _ = black_box(df_izip(&lf).unwrap());
    })
}

#[divan::bench]
fn bench_rayon(bencher: Bencher) {
    let lf = generate_df().unwrap();
    bencher.bench_local(move || {
        let lf = lf.clone();
        let _ = black_box(df_rayon(&lf).unwrap());
    })
}

fn generate_df() -> Result<DataFrame, PolarsError> {
    let name = "IS";
    let fname = "../data/airports.parquet";

    let expr = col("iso_country")
        .eq(lit(name))
        .and(col("iata_code").is_not_null());
    let expr = expr.clone();
    let res = LazyFrame::scan_parquet(fname.into(), Default::default())
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
        .collect()?;
    Ok(res)
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Airport {
    /// Airport identifier (ICAO code)
    pub ident: String,
    /// Full name of the airport
    pub name: String,
    /// Latitude in decimal degrees
    pub latitude_deg: f64,
    /// Longitude in decimal degrees
    pub longitude_deg: f64,
    /// Elevation in meters above sea level
    pub elevation_m: i32,
    /// IATA airport code
    pub iata_code: String,
    /// Timezone name (e.g., "Europe/Paris")
    pub timezone: String,
    /// UTC offset in hours
    pub offset: i32,
    /// Pluscode for the given coordinates
    pub pluscode: String,
}

fn compute_pluscode(latitude: f64, longitude: f64) -> eyre::Result<String> {
    let coord = Coordinate {
        latitude,
        longitude,
    };
    let pluscode = pluscodes::encode(&coord, 8)?;
    Ok(pluscode)
}

fn df_rayon(df: &DataFrame) -> PolarsResult<Vec<Airport>> {
    let ident = df.column("ident")?.str()?;
    let name = df.column("name")?.str()?;
    let lat = df.column("latitude_deg")?.f64()?;
    let lon = df.column("longitude_deg")?.f64()?;
    let elev = df.column("elevation_ft")?.i64()?;
    let iata = df.column("iata_code")?.str()?;

    let airports = (0..df.height())
        .into_par_iter()
        .map(|i| {
            let lat_val = lat.get(i).unwrap();
            let lon_val = lon.get(i).unwrap();
            let tzd = find_tz(lat_val, lon_val).unwrap();

            Airport {
                ident: ident.get(i).unwrap().to_string(),
                name: name.get(i).unwrap().to_string(),
                latitude_deg: lat_val,
                longitude_deg: lon_val,
                elevation_m: (elev.get(i).unwrap_or(0) as f64 * 0.3048) as i32,
                iata_code: iata.get(i).unwrap().to_string(),
                timezone: tzd.tzname,
                offset: tzd.offset / 3600,
                pluscode: compute_pluscode(lat_val, lon_val).unwrap(),
            }
        })
        .collect();

    Ok(airports)
}

fn df_izip(df: &DataFrame) -> PolarsResult<Vec<Airport>> {
    let ident = df.column("ident")?.str()?;
    let name = df.column("name")?.str()?;
    let lat = df.column("latitude_deg")?.f64()?;
    let lon = df.column("longitude_deg")?.f64()?;
    let elev = df.column("elevation_ft")?.i64()?;
    let iata = df.column("iata_code")?.str()?;

    let airports = izip!(
        ident.into_iter(),
        name.into_iter(),
        lat.into_iter(),
        lon.into_iter(),
        elev.into_iter(),
        iata.into_iter(),
    )
    .map(|(ident, name, lat, lon, elev, iata)| {
        // Calculate some more data for the struct
        //
        let tzd = find_tz(lat.unwrap(), lon.unwrap()).unwrap();
        let pluscode = compute_pluscode(lat.unwrap(), lon.unwrap()).unwrap();
        Airport {
            ident: ident.unwrap().to_string(),
            name: name.unwrap().to_string(),
            latitude_deg: lat.unwrap(),
            longitude_deg: lon.unwrap(),
            elevation_m: (elev.unwrap_or(0) as f64 * 0.3048) as i32,
            iata_code: iata.unwrap().to_string(),
            timezone: tzd.tzname,
            offset: tzd.offset / 3600,
            pluscode,
        }
    })
    .collect();
    Ok(airports)
}
