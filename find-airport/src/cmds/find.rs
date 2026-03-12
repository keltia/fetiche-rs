//! Airport module for finding airports by IATA code or name.
//!
//! Use opendata from OurAirports: https://ourairports.com/data/
//!
//! Get the csv file with curl and convert it into parquet:
//! ```shell
//! curl -O  https://davidmegginson.github.io/ourairports-data/airports.csv
//! bdt convert -s airports.csv airports.parquet
//! ```
//!

use std::path::Path;

use eyre::Result;
use itertools::izip;
use pluscodes::Coordinate;
use polars::error::PolarsResult;
use polars::frame::DataFrame;
use polars::prelude::*;
use serde::Deserialize;
use tabled::Tabled;
use tracing::info;

use fetiche_common::find_tz;

use crate::runtime::Context;

/// Represents an airport with its geographical and identification information.
///
/// This structure contains all relevant data for an airport including its location,
/// elevation, identification codes, and timezone information.
#[derive(Deserialize, Debug, Tabled)]
pub struct Airport {
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

/// Converts a Polars DataFrame into a vector of Airport structures.
///
/// Extracts airport data from the DataFrame columns and constructs Airport objects.
/// The elevation is converted from feet to meters, and timezone information is
/// determined based on geographical coordinates.
///
/// # Arguments
///
/// * `df` - A reference to a DataFrame containing airport data with columns:
///   ident, name, latitude_deg, longitude_deg, elevation_ft, and iata_code
///
/// # Returns
///
/// Returns a `PolarsResult` containing a vector of Airport objects on success,
/// or an error if the DataFrame structure is invalid.
#[tracing::instrument]
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

/// Finds airports matching a given IATA code or name.
///
/// Searches the airport database (stored as a Parquet file) for airports that match
/// either the IATA code exactly or contain the search string in their name.
/// Only airports with a valid IATA code are returned.
///
/// # Arguments
///
/// * `name` - A string slice containing either an IATA code or partial airport name
///
/// # Returns
///
/// Returns a Result containing a vector of matching Airport objects, or an error
/// if the database cannot be read or queried.
///
/// # Note
///
/// This function is compatible with Polars 0.52 and older versions.
///
#[tracing::instrument]
pub fn cmd_find(ctx: &Context, name: &str) -> Result<Vec<Airport>> {
    let basedir = ctx.cfg["datalake"].clone();
    info!("Datalake is {}", basedir);
    let fname = Path::new(&basedir).join("files").join("airports.parquet");
    info!("Looking for airports in {}", fname.display());
    let fname = PlPath::Local(fname.into());

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

const DEF_LENGTH: usize = 8;

#[tracing::instrument]
fn compute_pluscode(latitude: f64, longitude: f64) -> Result<String> {
    let coord = Coordinate {
        latitude,
        longitude,
    };
    let pluscode = pluscodes::encode(&coord, DEF_LENGTH)?;
    Ok(pluscode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("KJFK", "John F Kennedy International Airport", 40.639751, -73.778925)]
    #[case("LFPG", "Charles de Gaulle International Airport", 49.012779, 2.55)]
    #[case("RJTT", "Tokyo International Airport", 35.552258, 139.779694)]
    #[case("EGLL", "London Heathrow Airport", 51.4706, -0.461941)]
    fn test_find_airport_known_examples(
        #[case] iata: &str,
        #[case] expected_name: &str,
        #[case] expected_lat: f64,
        #[case] expected_lon: f64,
    ) {
        let result = cmd_find(iata);
        assert!(result.is_ok());
        let airport = result.unwrap().first().unwrap();
        assert_eq!(airport.iata_code, iata);
        assert!(airport.name.contains(expected_name) || expected_name.contains(&airport.name));
        // Allow small tolerance for coordinate comparison (0.01 degrees ~= 1km)
        assert!((airport.latitude_deg - expected_lat).abs() < 0.01);
        assert!((airport.longitude_deg - expected_lon).abs() < 0.01);
        // Verify altitude is reasonable (between -500m and 5000m for most airports)
        assert!(airport.elevation_m >= -500 && airport.elevation_m <= 5000);
    }
}
