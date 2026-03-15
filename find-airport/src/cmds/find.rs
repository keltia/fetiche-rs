//! Airport search functionality for finding airports by various criteria.
//!
//! This module provides a flexible airport search system that queries airport data
//! stored in Parquet format. It supports searching by IATA code, ICAO identifier,
//! airport name, or country code, and enriches results with timezone information
//! and Plus Codes for precise location identification.
//!
//! # Data Source
//!
//! Uses opendata from OurAirports: https://ourairports.com/data/
//!
//! # Setup
//!
//! Get the CSV file with curl and convert it into Parquet format:
//! ```shell
//! curl -O  https://davidmegginson.github.io/ourairports-data/airports.csv
//! bdt convert -s airports.csv airports.parquet
//! ```
//!
//! # Workflow
//!
//! 1. Load airport data from Parquet file in the configured datalake
//! 2. Filter airports based on the selected search criteria
//! 3. Enrich results with timezone information using coordinates
//! 4. Generate Plus Codes for precise geolocation
//! 5. Convert elevation from feet to meters
//! 6. Return structured Airport objects with complete information
//!

use eyre::Result;
use pluscodes::Coordinate;
use polars::prelude::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use tabled::Tabled;
use tracing::{info, trace};

use crate::cli::FindOpts;
use crate::runtime::Context;
use fetiche_common::find_tz;

/// Represents an airport with its geographical and identification information.
///
/// This structure contains all relevant data for an airport including its location,
/// elevation, identification codes, and timezone information.
///
#[derive(Deserialize, Debug, Serialize, Tabled)]
pub struct Airport {
    /// Airport identifier (ICAO code)
    #[tabled(rename = "ICAO")]
    pub ident: String,
    /// Full name of the airport
    #[tabled(rename = "Name")]
    pub name: String,
    /// Latitude in decimal degrees
    #[tabled(rename = "Latitude", format("{:.6}"))]
    pub latitude_deg: f64,
    /// Longitude in decimal degrees
    #[tabled(rename = "Longitude", format("{:.6}"))]
    pub longitude_deg: f64,
    /// Elevation in meters above sea level
    #[tabled(rename = "Elevation (m)")]
    pub elevation_m: i32,
    /// IATA airport code
    #[tabled(rename = "IATA")]
    pub iata_code: String,
    /// Timezone name (e.g., "Europe/Paris")
    #[tabled(rename = "Timezone")]
    pub timezone: String,
    /// UTC offset in hours
    #[tabled(rename = "UTC Offset")]
    pub offset: i32,
    /// Pluscode for the given coordinates
    #[tabled(rename = "Pluscode")]
    pub pluscode: String,
}

/// Defines the search strategy for querying the airport database.
///
/// This enum determines which field of the airport data will be used
/// for filtering results. Each variant corresponds to a different
/// search approach with varying levels of exactness.
#[derive(Copy, Clone, Debug, Default)]
enum SearchBy {
    /// Search by ISO country code (partial match, case-insensitive)
    Country,
    /// Search by IATA airport code (exact match, default strategy)
    #[default]
    Iata,
    /// Search by ICAO identifier (partial match, case-insensitive)
    Icao,
    /// Search by airport name (partial match, case-insensitive)
    Name,
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
#[tracing::instrument(skip(ctx))]
pub fn cmd_find(ctx: &Context, opts: &FindOpts) -> Result<Vec<Airport>> {
    let basedir = ctx.cfg["datalake"].clone();
    info!("datalake={}", basedir);

    let name = opts.text.as_str();
    let fname = Path::new(&basedir).join("files").join("airports.parquet");
    let fname = fname.to_string_lossy().to_string();
    info!("find={} airports={}", name, fname);

    // Default is IATA code, but can be overriden by --icao / --name / --country
    //
    let criteria = if opts.country {
        SearchBy::Country
    } else if opts.icao {
        SearchBy::Icao
    } else if opts.name {
        SearchBy::Name
    } else {
        SearchBy::Iata
    };

    let tm = Instant::now();
    let lf = find_into_parquet(name, &fname, criteria)?;
    let tm = tm.elapsed().as_millis();
    trace!("find={tm}ms, nrows={}", lf.height());

    // let airports: Vec<Airport> = lf.deserialize()?;
    // This is the slowest part of the processing
    //
    let tm = Instant::now();
    let airports = airports_from_df(&lf)?;
    let tm = tm.elapsed().as_millis();
    trace!("deserialize={tm}ms, nrows={}", airports.len());

    Ok(airports)
}

// -----

/// Queries the Parquet file for airports matching the specified search criteria.
///
/// This function scans the airport database stored in Parquet format and filters
/// results based on the provided search criteria. The search can be performed by:
/// - Country: Partial match on ISO country code (case-insensitive)
/// - IATA: Exact match on IATA airport code
/// - ICAO: Partial match on ICAO identifier (case-insensitive)
/// - Name: Partial match on airport name (case-insensitive)
///
/// Only airports with a valid (non-null) IATA code are included in the results.
///
/// # Arguments
///
/// * `name` - The search string to match against (interpretation depends on criteria)
/// * `fname` - Path to the Parquet file containing airport data
/// * `criteria` - The search strategy to use (Country, Iata, Icao, or Name)
///
/// # Returns
///
/// Returns a `Result` containing a `DataFrame` with columns: ident, name, latitude_deg,
/// longitude_deg, elevation_ft, iata_code, and iso_country. Returns an error if the
/// file cannot be read or the query fails.
///
/// # Note
///
/// The function uses lazy evaluation and only materializes results when `collect()` is called.
///
#[tracing::instrument]
fn find_into_parquet(name: &str, fname: &str, criteria: SearchBy) -> Result<DataFrame> {
    info!("find={} airports={}", name, fname);

    let expr = match criteria {
        SearchBy::Country => col("iso_country").str().contains(lit(name), true),
        SearchBy::Iata => col("iata_code").eq(lit(name)),
        SearchBy::Icao => col("ident").str().contains(lit(name), true),
        SearchBy::Name => col("name").str().contains(lit(name), true),
    };
    let lf = LazyFrame::scan_parquet(fname.into(), Default::default())?
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
        .filter(col("iata_code").is_not_null())
        .collect()?;
    Ok(lf)
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
///
#[tracing::instrument]
fn airports_from_df(df: &DataFrame) -> PolarsResult<Vec<Airport>> {
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

/// Default length for Plus Code generation (8 digits plus separator).
///
/// This produces codes with approximately 14m x 14m resolution, which is
/// suitable for airport location precision. The resulting code will be
/// 9 characters including the '+' separator (e.g., "87G8J6QC+").
///
const DEF_LENGTH: usize = 8;

/// Generates a Plus Code (Open Location Code) for the given geographic coordinates.
///
/// Plus Codes are short codes that can be used like street addresses, for places
/// where street addresses don't exist. This function creates an 8-digit Plus Code
/// (with '+' separator), providing approximately 14m x 14m resolution, which is
/// sufficient for identifying airport locations precisely.
///
/// # Arguments
///
/// * `latitude` - The latitude in decimal degrees (range: -90.0 to 90.0)
/// * `longitude` - The longitude in decimal degrees (range: -180.0 to 180.0)
///
/// # Returns
///
/// Returns a `Result` containing a 9-character string representing the Plus Code
/// (e.g., "87G8J6QC+") on success, or an error if the coordinates are invalid.
///
/// # Format
///
/// The returned Plus Code consists of:
/// - 8 alphanumeric characters (using digits 2-9 and letters C-X, excluding vowels)
/// - 1 '+' separator at position 8
/// - Total length: 9 characters
///
/// # Examples
///
/// ```no_run
/// # use eyre::Result;
/// # fn compute_pluscode(latitude: f64, longitude: f64) -> Result<String> { Ok(String::new()) }
/// // JFK Airport coordinates
/// let pluscode = compute_pluscode(40.639751, -73.778925)?;
/// assert_eq!(pluscode, "87G8J6QC+");
/// # Ok::<(), eyre::Error>(())
/// ```
///
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
    #[case("JFK", "Kennedy International")]
    #[case("CDG", "Charles de Gaulle")]
    #[case("LAX", "Los Angeles International")]
    #[case("LHR", "London Heathrow")]
    fn test_find_airport_known_examples(
        #[case] iata: &str,
        #[case] expected_name_part: &str,
    ) -> Result<()> {
        let result = find_into_parquet(iata, "../data/airports.parquet", SearchBy::Iata);
        assert!(result.is_ok());
        let result = result.unwrap();
        let airports = airports_from_df(&result)?;

        let airport = airports.first().unwrap();
        assert_eq!(airport.iata_code, iata);
        assert!(
            airport.name.contains(expected_name_part) || expected_name_part.contains(&airport.name)
        );
        // Verify altitude is reasonable (between -500m and 5000m for most airports)
        assert!(airport.elevation_m >= -500 && airport.elevation_m <= 5000);
        // Verify pluscode is generated
        assert!(!airport.pluscode.is_empty());
        assert_eq!(airport.pluscode.len(), 9);
        Ok(())
    }

    #[rstest]
    #[case(40.639751, -73.778925, "87G8J6QC+")] // JFK Airport
    #[case(49.012779, 2.55, "8FX42H72+")] // Charles de Gaulle
    #[case(35.552258, 139.779694, "8Q7XHQ2H+")] // Tokyo International
    #[case(51.4706, -0.461941, "9C3XFGCQ+")] // Heathrow
    #[case(0.0, 0.0, "6FG22222+")] // Null Island
    #[case(47.123456, 8.123456, "8FVC44FF+")] // Arbitrary location
    fn test_compute_pluscode(
        #[case] latitude: f64,
        #[case] longitude: f64,
        #[case] expected_code: &str,
    ) -> Result<()> {
        let result = compute_pluscode(latitude, longitude)?;
        assert_eq!(result.len(), 9); // 8 chars + '+'
        assert_eq!(result, expected_code);
        Ok(())
    }
}
