//! Try find a specific airport by IATA code.
//!
//! Use opendata from OurAirports: https://ourairports.com/data/
//!
//! Get the csv file with curl, and convert into parquet:
//! ```text
//! curl -O  https://davidmegginson.github.io/ourairports-data/airports.csv
//! bdt convert -s airports.csv airports.parquet
//! ```
//!
use clap::Parser;
use eyre::Result;
use fetiche_common::find_tz;
use itertools::izip;
use polars::prelude::*;
use serde::Deserialize;
use tabled::settings::Style;
use tabled::{Table, Tabled};

/// Command-line options for the airport lookup application.
///
/// Contains the search parameter for finding airports by IATA code or name.
#[derive(Debug, clap::Parser)]
struct Opts {
    /// Airport IATA code or name to search for (default: "CDG")
    #[clap(default_value = "CDG")]
    name: String,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    println!("Looking for airport: {}", &opts.name);

    let airport_iata = find_airport(&opts.name)?;

    let table = Table::new(&airport_iata).with(Style::sharp()).to_string();
    println!("Found by IATA:\n{table}");
    Ok(())
}

#[derive(Deserialize, Debug, Tabled)]
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
}

fn airports_from_df(df: &DataFrame) -> PolarsResult<Vec<Airport>> {
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
        let tzd = find_tz(lat.unwrap(), lon.unwrap()).unwrap();

        Airport {
            ident: ident.unwrap().to_string(),
            name: name.unwrap().to_string(),
            latitude_deg: lat.unwrap(),
            longitude_deg: lon.unwrap(),
            elevation_m: (elev.unwrap_or(0) as f64 * 0.3048) as i32,
            iata_code: iata.unwrap_or("").to_string(),
            timezone: tzd.tzname,
            offset: tzd.offset / 3600,
        }
    })
    .collect();

    Ok(airports)
}

fn find_airport(name: &str) -> Result<Vec<Airport>> {
    let fname = "../data/airports.parquet";

    let fname = PlPath::from_str(fname);
    let lf = LazyFrame::scan_parquet(fname, Default::default())?
        .select([
            col("ident"),
            col("name"),
            col("latitude_deg"),
            col("longitude_deg"),
            col("elevation_ft"),
            col("iata_code"),
        ])
        .filter(
            col("iata_code")
                .eq(lit(name))
                .or(col("name").str().contains(lit(name), true))
                .or(col("ident").str().contains(lit(name), true)),
        )
        .filter(col("iata_code").is_not_null())
        .collect()?;

    // This is only work in 0.53 (aka WHEN THEY FIX 0.53 which is broken with regard to chrono)
    // FML.
    // let airports: Vec<Airport> = lf.deserialize()?;
    let airports = airports_from_df(&lf)?;
    Ok(airports)
}
