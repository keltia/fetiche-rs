//! Export the distances calculated by the `distances` module.
//!

use std::fmt::Debug;
use std::fs;

use clap::Parser;
use eyre::Result;
use klickhouse::{DateTime, Row};
use polars::prelude::{DataFrame, ParquetWriter, df};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace};

use fetiche_common::Delim;

use crate::cmds::{CmdError, DBVars, Format};
use crate::make_query;
use crate::runtime::Context;

#[derive(Debug, Parser)]
pub struct ExpDistOpts {
    /// Summary or everything?
    #[clap(short = 'S', long)]
    pub summary: bool,
    /// Output format
    #[clap(short = 'F', long, default_value = "csv", value_parser)]
    pub format: Format,
    /// Output file
    #[clap(short = 'o', long)]
    pub output: Option<String>,
}

/// Represents an individual encounter record with detailed information.
///
/// This struct is used to deserialize data retrieved from the `airplane_prox` table
/// into a structured format.
///
/// # Fields
///
/// - `site` - Identifier of the site where the encounter occurred.
/// - `sitename` - Name of the site where the encounter occurred.
/// - `en_id` - Unique ID of the encounter.
/// - `time` - Timestamp of when the encounter happened.
/// - `journey` - Journey identifier associated with the encounter.
/// - `drone_id` - Unique identifier of the drone involved in the encounter.
/// - `model` - Model name of the drone.
/// - `drone_lat` - Latitude coordinate of the drone during the encounter.
/// - `drone_lon` - Longitude coordinate of the drone during the encounter.
/// - `drone_alt_m` - Altitude of the drone in meters.
/// - `drone_height_m` - Height of the drone above ground level in meters.
/// - `prox_callsign` - Callsign of the proximal aircraft involved in the encounter.
/// - `prox_id` - Identifier of the proximal aircraft.
/// - `prox_lat` - Latitude coordinate of the proximal aircraft.
/// - `prox_lon` - Longitude coordinate of the proximal aircraft.
/// - `prox_alt_m` - Altitude of the proximal aircraft in meters.
/// - `prox_mode_a` - Squawk code of the aircraft.
/// - `distance_slant_m` - Slant distance between the drone and proximal aircraft in meters.
/// - `distance_hor_m` - Horizontal distance between the drone and proximal aircraft in meters.
/// - `distance_vert_m` - Vertical distance between the drone and proximal aircraft in meters.
/// - `distance_home_m` - Distance between the drone and its home location in meters.
///
#[derive(Debug, Deserialize, Row, Serialize)]
struct Encounter {
    site: i32,
    sitename: String,
    en_id: String,
    time: DateTime,
    journey: i32,
    drone_id: String,
    model: String,
    drone_lat: f32,
    drone_lon: f32,
    drone_alt_m: f32,
    drone_height_m: f32,
    prox_callsign: String,
    prox_id: String,
    prox_lat: f32,
    prox_lon: f32,
    prox_alt_m: f32,
    prox_mode_a: String,
    prox_ecat: i32,
    distance_slant_m: i32,
    distance_hor_m: i32,
    distance_vert_m: i32,
    distance_home_m: i32,
}

/// Retrieves all the encounter records from the `airplane_prox` table in the database.
///
/// The function executes a database query to select all records and orders them
/// by time for structured representation. The retrieved data is deserialized into
/// a vector of `Encounter` structs.
///
/// # Arguments
/// * `ctx` - Application context providing access to the database and other resources.
///
/// # Returns
/// * `Result<Vec<Encounter>>` - Returns a vector of `Encounter` structs with the
///   data fetched from the table upon successful execution. If an error occurs
///   (e.g., query execution or deserialization failure), it returns an error type.
///
/// # Process
/// 1. Executes a SQL query to retrieve all encounter records in the `airplane_prox` table.
/// 2. Orders the records by time.
/// 3. Collects and deserializes the records into the `Encounter` struct format.
///
/// # Errors
/// This function may return errors in the following scenarios:
///
/// * Database connection or query errors while fetching the records.
/// * Data deserialization issues while converting query rows into the `Encounter` struct format.
///
/// # Examples
/// ```rust,no_run
/// # use process_data::{Context, ExpDistOpts, Format, export_results};
/// async fn example(ctx: &Context) -> eyre::Result<()> {
///     let opts = ExpDistOpts {
///         summary: false,
///         format: Format::Csv,
///         output: Some("encounters.csv".to_string()),
///     };
///     export_results(ctx, &opts).await
/// }
/// ```
///
#[tracing::instrument(skip(ctx))]
async fn retrieve_all_encounters(ctx: &Context) -> Result<Vec<Encounter>> {
    let client = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(
        r##"
  SELECT
    site_id,
    sitename,
    en_id,
    time,
    journey,
    drone_id,
    model,
    drone_lat,
    drone_lon,
    drone_alt_m,
    drone_height_m,
    prox_callsign,
    prox_id,
    prox_lat,
    prox_lon,
    prox_alt_m,
    prox_mode_a,
    prox_ecat,
    distance_slant_m,
    distance_hor_m,
    distance_vert_m,
    distance_home_m
  FROM {workdb}.airplane_prox
  ORDER BY time
        "##,
        dbvars
    );

    let res = client.query_collect::<Encounter>(&r).await?;
    debug!("retrieved encounters: {:?}", res);

    Ok(res)
}

/// This function retrieves a summarized subset of encounter records from the `airplane_prox` table.
/// The summary data is derived by joining the `airplane_prox` table with the `airprox_summary` table
/// on specific matching criteria. The resulting records are ordered by the encounter time before being returned.
///
/// # Arguments
/// * `ctx` - Application context providing access to the database and other resources.
///
/// # Returns
/// * `Result<Vec<Encounter>>` - Returns a vector of `Encounter` structs containing the summary data
///   upon successful execution, or an error if the query fails or data deserialization is unsuccessful.
///
/// # Process
/// 1. Executes a query joining the `airplane_prox` and `airprox_summary` tables to fetch the summarized data.
/// 2. Filters the data based on matching criteria such as encounter ID, journey, drone ID, and distance.
/// 3. Orders the resulting records by time and collects them into `Encounter` structs.
///
/// # Errors
/// This function may return errors in the following cases:
/// * Database query errors while fetching records.
/// * Deserialization errors during mapping query results into the `Encounter` struct.
///
#[tracing::instrument(skip(ctx))]
async fn retrieve_summary_encounters(ctx: &Context) -> Result<Vec<Encounter>> {
    let client = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    // Match with airprox_summary for export
    //
    let r1 = make_query!(
        r##"
  SELECT
    site_id,
    sitename,
    en_id,
    time,
    journey,
    drone_id,
    model,
    drone_lat,
    drone_lon,
    drone_alt_m,
    drone_height_m,
    prox_callsign,
    prox_id,
    prox_lat,
    prox_lon,
    prox_alt_m,
    prox_mode_a,
    prox_ecat,
    distance_slant_m,
    distance_hor_m,
    distance_vert_m,
    distance_home_m
  FROM
    {workdb}.airplane_prox AS a JOIN {workdb}.airprox_summary AS s
    ON
        s.en_id = a.en_id AND
        s.journey = a.journey AND
        s.drone_id = a.drone_id
  WHERE
    a.distance_slant_m = s.distance_slant_m
  ORDER BY time
    "##,
        dbvars
    );

    let summ = client.query_collect::<Encounter>(&r1).await?;
    trace!("Summary encounters: {:?}", summ);
    Ok(summ)
}

/// Exports all encounter records from the `airplane_prox` table into a CSV file.
///
/// This function retrieves all records in the `airplane_prox` table using the `retrieve_all_encounters`
/// function and serializes them into a CSV file specified by the `fname` argument.
///
/// # Arguments
/// * `ctx` - Application context providing access to the database and other resources.
/// * `fname` - The output file path where the CSV data will be written.
///
/// # Returns
/// * `Result<()>` - Returns an `Ok` result on successful execution, or an error if anything goes wrong.
///
/// # Process
/// 1. Retrieves all encounters from the `airplane_prox` table.
/// 2. Serializes the records into CSV format using the `csv` crate.
/// 3. Writes the serialized CSV data into the specified file.
///
/// # Errors
/// This function may return errors in the following cases:
/// * Database query errors while retrieving encounter records.
/// * File I/O errors during CSV writing.
/// * Data serialization errors while converting records into CSV format.
///
/// # Examples
/// ```rust,no_run
/// # use process_data::{Context, ExpDistOpts, Format, export_results};
/// async fn example(ctx: &Context) -> eyre::Result<()> {
///     let opts = ExpDistOpts {
///         summary: false,
///         format: Format::Csv,
///         output: Some("encounters.csv".to_string()),
///     };
///     export_results(ctx, &opts).await
/// }
/// ```
///
#[tracing::instrument(skip(ctx))]
async fn export_all_encounters_csv(ctx: &Context, fname: &str) -> Result<()> {
    let data = retrieve_all_encounters(ctx).await?;
    let len = data.len();

    let data = Delim::Comma.prepare_csv(&data, true)?;

    fs::write(fname, data)?;
    trace!("Exported {} encounters", len);

    Ok(())
}

/// Convert a slice of `Encounter` records directly into a Polars `DataFrame` in memory.
fn encounters_to_df(encounters: &[Encounter]) -> Result<DataFrame> {
    let mut site: Vec<i32> = Vec::with_capacity(encounters.len());
    let mut sitename: Vec<String> = Vec::with_capacity(encounters.len());
    let mut en_id: Vec<String> = Vec::with_capacity(encounters.len());
    let mut time: Vec<String> = Vec::with_capacity(encounters.len());
    let mut journey: Vec<i32> = Vec::with_capacity(encounters.len());
    let mut drone_id: Vec<String> = Vec::with_capacity(encounters.len());
    let mut model: Vec<String> = Vec::with_capacity(encounters.len());
    let mut drone_lat: Vec<f32> = Vec::with_capacity(encounters.len());
    let mut drone_lon: Vec<f32> = Vec::with_capacity(encounters.len());
    let mut drone_alt_m: Vec<f32> = Vec::with_capacity(encounters.len());
    let mut drone_height_m: Vec<f32> = Vec::with_capacity(encounters.len());
    let mut prox_callsign: Vec<String> = Vec::with_capacity(encounters.len());
    let mut prox_id: Vec<String> = Vec::with_capacity(encounters.len());
    let mut prox_lat: Vec<f32> = Vec::with_capacity(encounters.len());
    let mut prox_lon: Vec<f32> = Vec::with_capacity(encounters.len());
    let mut prox_alt_m: Vec<f32> = Vec::with_capacity(encounters.len());
    let mut prox_mode_a: Vec<String> = Vec::with_capacity(encounters.len());
    let mut prox_ecat: Vec<i32> = Vec::with_capacity(encounters.len());
    let mut distance_slant_m: Vec<i32> = Vec::with_capacity(encounters.len());
    let mut distance_hor_m: Vec<i32> = Vec::with_capacity(encounters.len());
    let mut distance_vert_m: Vec<i32> = Vec::with_capacity(encounters.len());
    let mut distance_home_m: Vec<i32> = Vec::with_capacity(encounters.len());

    for e in encounters {
        site.push(e.site);
        sitename.push(e.sitename.clone());
        en_id.push(e.en_id.clone());
        let ts =
            jiff::Timestamp::from_second(e.time.1 as i64).unwrap_or(jiff::Timestamp::UNIX_EPOCH);
        let time_str = if let Ok(tz) = jiff::tz::TimeZone::get(e.time.0.name()) {
            ts.to_zoned(tz).to_string()
        } else {
            ts.to_string()
        };
        time.push(time_str);
        journey.push(e.journey);
        drone_id.push(e.drone_id.clone());
        model.push(e.model.clone());
        drone_lat.push(e.drone_lat);
        drone_lon.push(e.drone_lon);
        drone_alt_m.push(e.drone_alt_m);
        drone_height_m.push(e.drone_height_m);
        prox_callsign.push(e.prox_callsign.clone());
        prox_id.push(e.prox_id.clone());
        prox_lat.push(e.prox_lat);
        prox_lon.push(e.prox_lon);
        prox_alt_m.push(e.prox_alt_m);
        prox_mode_a.push(e.prox_mode_a.clone());
        prox_ecat.push(e.prox_ecat);
        distance_slant_m.push(e.distance_slant_m);
        distance_hor_m.push(e.distance_hor_m);
        distance_vert_m.push(e.distance_vert_m);
        distance_home_m.push(e.distance_home_m);
    }

    let df = df!(
        "site" => site,
        "sitename" => sitename,
        "en_id" => en_id,
        "time" => time,
        "journey" => journey,
        "drone_id" => drone_id,
        "model" => model,
        "drone_lat" => drone_lat,
        "drone_lon" => drone_lon,
        "drone_alt_m" => drone_alt_m,
        "drone_height_m" => drone_height_m,
        "prox_callsign" => prox_callsign,
        "prox_id" => prox_id,
        "prox_lat" => prox_lat,
        "prox_lon" => prox_lon,
        "prox_alt_m" => prox_alt_m,
        "prox_mode_a" => prox_mode_a,
        "prox_ecat" => prox_ecat,
        "distance_slant_m" => distance_slant_m,
        "distance_hor_m" => distance_hor_m,
        "distance_vert_m" => distance_vert_m,
        "distance_home_m" => distance_home_m,
    )?;

    Ok(df)
}

/// Exports all encounters as a Parquet file.
///
/// This function retrieves all records in the `airplane_prox` table, converts them
/// directly into a Polars DataFrame in memory, and writes the DataFrame to a Parquet file.
///
/// # Arguments
/// * `ctx` - Application context providing access to the database and other resources.
/// * `fname` - The output filename where the Parquet file will be saved.
///
/// # Returns
/// * `Result<()>` - Returns an `Ok` result on success, or an error if anything goes wrong.
///
/// # Process
/// 1. Retrieves all encounters from the `airplane_prox` table.
/// 2. Converts the records directly in-memory into a Polars `DataFrame`.
/// 3. Writes the DataFrame into a Parquet file format.
///
/// # Errors
/// This function may return errors in the following cases:
/// * Issues with writing Parquet files.
/// * Database query errors while retrieving the encounters data.
///
/// # Examples
/// ```rust,no_run
/// # use process_data::{Context, ExpDistOpts, Format, export_results};
/// async fn example(ctx: &Context) -> eyre::Result<()> {
///     let opts = ExpDistOpts {
///         summary: false,
///         format: Format::Parquet,
///         output: Some("output.parquet".to_string()),
///     };
///     export_results(ctx, &opts).await
/// }
/// ```
///
#[tracing::instrument(skip(ctx))]
async fn export_all_encounters_parquet(ctx: &Context, fname: &str) -> Result<()> {
    let data = retrieve_all_encounters(ctx).await?;
    let len = data.len();

    trace!("Writing {fname} as parquet with {len} encounters.");

    let mut df = encounters_to_df(&data)?;

    let mut file = fs::File::create(fname)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;

    eprintln!("Summary file {fname}");

    Ok(())
}

/// Exports a summary of all encounters for a specific day as a CSV file.
///
/// This function retrieves summary data about encounters from the database,
/// formats it as a CSV file, and writes it to a specified output filename.
///
/// # Arguments
/// * `ctx` - Application context providing access to the database and other resources.
/// * `fname` - The output filename where the CSV file will be saved.
///
/// # Returns
/// * `eyre::Result<()>` - Returns an `Ok` result on success, or an error if something fails.
///
/// # Errors
/// This function may return errors in the following cases:
/// * Failure to retrieve summary data from the database.
/// * Issues with writing the data to the specified CSV file.
///
#[tracing::instrument(skip(ctx))]
async fn export_all_encounters_summary_csv(ctx: &Context, fname: &str) -> eyre::Result<()> {
    // Create a temp file with all min distances
    //
    let data = retrieve_summary_encounters(ctx).await?;
    let len = data.len();

    let data = Delim::Comma.prepare_csv(&data, true)?;

    fs::write(fname, data)?;
    trace!("Exported {} encounters", len);

    Ok(())
}

/// Main entry point for the various `export distances` subcommand.
///
/// This function handles exporting of distance-related data based on the provided options.
/// It interacts with the database through the application context and performs various
/// export operations, such as generating summary reports or exporting detailed data
/// in different formats (e.g., CSV, Parquet).
///
/// # Arguments
/// * `ctx` - Application context providing access to the database and other resources.
/// * `opts` - Options specifying the export details, such as output format and summary.
///
/// # Returns
/// * `eyre::Result<()>` - Returns an `Ok` result on success or an error in case of failure.
///
/// # Process
/// * Connects to the database using the provided context.
/// * Determines whether to export a summary or detailed data based on the `opts`.
/// * Supports multiple output formats including CSV and Parquet.
/// * Writes the results to the specified file if the `output` option is provided.
///
/// # Errors
/// This function may return errors in the following cases:
/// * Missing output destination.
/// * Unsupported export format.
/// * Errors during database queries, file writing, or data serialization.
///
/// # Notes
/// * Ensure the database connection is available before invoking this function.
/// * The output file name and format should be specified in the options.
///
/// TODO: Fix output format handling for more formats.
///
#[tracing::instrument(skip(ctx))]
pub async fn export_results(ctx: &Context, opts: &ExpDistOpts) -> eyre::Result<()> {
    // Do we export as a csv the "encounters of the day"?
    //
    match &opts.output {
        Some(fname) => {
            if opts.summary {
                export_all_encounters_summary_csv(ctx, fname).await?
            } else {
                match opts.format {
                    Format::Csv => export_all_encounters_csv(ctx, fname).await?,
                    Format::Parquet => export_all_encounters_parquet(ctx, fname).await?,
                    _ => {
                        return {
                            eprintln!("Unknown format specified.");
                            Err(CmdError::UnknownFormat(opts.format.to_string()).into())
                        };
                    }
                }
            };
        }
        None => {
            eprintln!("No output file specified.");
            return Err(CmdError::NoOutputDestination.into());
        }
    }
    info!("Done.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encounters_to_df_empty() {
        let empty: Vec<Encounter> = Vec::new();
        let df = encounters_to_df(&empty).expect("should build dataframe for empty encounters");
        assert_eq!(df.height(), 0);
        assert_eq!(df.width(), 22);
        assert!(df.column("site").is_ok());
        assert!(df.column("sitename").is_ok());
        assert!(df.column("en_id").is_ok());
        assert!(df.column("time").is_ok());
        assert!(df.column("distance_home_m").is_ok());
    }

    #[test]
    fn test_encounters_to_df_populated_and_parquet_write() {
        let time = DateTime(klickhouse::Tz::UTC, 1704110400);

        let encounter = Encounter {
            site: 1,
            sitename: "SiteA".to_string(),
            en_id: "ENC001".to_string(),
            time,
            journey: 42,
            drone_id: "DRONE01".to_string(),
            model: "Mavic".to_string(),
            drone_lat: 48.85,
            drone_lon: 2.35,
            drone_alt_m: 120.0,
            drone_height_m: 50.0,
            prox_callsign: "AFR123".to_string(),
            prox_id: "PROX01".to_string(),
            prox_lat: 48.86,
            prox_lon: 2.36,
            prox_alt_m: 300.0,
            prox_mode_a: "7000".to_string(),
            prox_ecat: 1,
            distance_slant_m: 1500,
            distance_hor_m: 1400,
            distance_vert_m: 180,
            distance_home_m: 2500,
        };

        let encounters = vec![encounter];
        let mut df = encounters_to_df(&encounters).expect("should convert encounters to df");
        assert_eq!(df.height(), 1);
        assert_eq!(df.width(), 22);

        let site_col = df.column("site").unwrap();
        assert_eq!(site_col.get(0).unwrap(), polars::prelude::AnyValue::Int32(1));

        let temp_dir = tempfile::tempdir().unwrap();
        let parquet_path = temp_dir.path().join("test_encounters.parquet");
        let mut file = fs::File::create(&parquet_path).unwrap();
        ParquetWriter::new(&mut file).finish(&mut df).unwrap();

        assert!(parquet_path.exists());
        assert!(fs::metadata(&parquet_path).unwrap().len() > 0);
    }
}
