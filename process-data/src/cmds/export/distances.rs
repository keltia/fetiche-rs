//! Export the distances calculated by the `distances` module.
//!

use std::fmt::Debug;
use std::fs;

use clap::Parser;
use csv::WriterBuilder;
use eyre::Result;
use klickhouse::{Client, DateTime, Row};
use polars::io::SerReader;
use polars::prelude::{CsvParseOptions, ParquetWriter};
use serde::{Deserialize, Serialize};
use tempfile::Builder;
use tracing::{debug, info, trace};

use crate::cmds::Format;
use crate::error::Status;
use crate::runtime::Context;

#[derive(Debug, Parser)]
pub struct ExpDistOpts {
    /// Summary or everything?
    #[clap(short = 'S', long)]
    pub summary: bool,
    /// Output format
    #[clap(short = 'F', long, default_value = "csv")]
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
///
/// * `client` - A reference to the database client used to execute the query.
///
/// # Returns
///
/// * `Result<Vec<Encounter>>` - Returns a vector of `Encounter` structs with the
///   data fetched from the table upon successful execution. If an error occurs
///   (e.g., query execution or deserialization failure), it returns an error type.
///
/// # Process
///
/// 1. Executes a SQL query to retrieve all encounter records in the `airplane_prox` table.
/// 2. Orders the records by time.
/// 3. Collects and deserializes the records into the `Encounter` struct format.
///
/// # Errors
///
/// This function may return errors in the following scenarios:
///
/// * Database connection or query errors while fetching the records.
/// * Data deserialization issues while converting query rows into the `Encounter` struct format.
///
/// # Examples
///
/// ```rust
/// use klickhouse::Client;
/// use eyre::Result;
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = Client::default();
///
///     let encounters = retrieve_all_encounters(&client).await?;
///     println!("Retrieved {} encounters", encounters.len());
///
///     Ok(())
/// }
/// ```
///
#[tracing::instrument(skip(client))]
async fn retrieve_all_encounters(client: &Client) -> Result<Vec<Encounter>> {
    trace!("retrieving records from airplane_prox");

    let r = r##"
  SELECT
    site,
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
    distance_slant_m,
    distance_hor_m,
    distance_vert_m,
    distance_home_m,
  FROM airplane_prox
  ORDER BY time
        "##;

    let res = client.query_collect::<Encounter>(r).await?;
    debug!("retrieved encounters: {:?}", res);

    Ok(res)
}

/// This function retrieves a summarized subset of encounter records from the `airplane_prox` table.
/// The summary data is derived by joining the `airplane_prox` table with the `airprox_summary` table
/// on specific matching criteria. The resulting records are ordered by the encounter time before being returned.
///
/// # Arguments
/// * `client` - A reference to the database client used to execute the query.
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
#[tracing::instrument(skip(client))]
async fn retrieve_summary_encounters(client: &Client) -> Result<Vec<Encounter>> {
    trace!("retrieving summary records from airplane_prox");

    // Match with airprox_summary for export
    //
    let r1 = r##"
  SELECT *
  FROM
    airplane_prox AS a JOIN airprox_summary AS s
    ON
        s.en_id = a.en_id AND
        s.journey = a.journey AND
        s.drone_id = a.drone_id
  WHERE
    a.distance_slant_m = s.distance_slant_m
  ORDER BY time
    "##;
    let summ = client.query_collect::<Encounter>(r1).await?;
    trace!("Summary encounters: {:?}", summ);
    Ok(summ)
}

/// Exports all encounter records from the `airplane_prox` table into a CSV file.
///
/// This function retrieves all records in the `airplane_prox` table using the `retrieve_all_encounters`
/// function and serializes them into a CSV file specified by the `fname` argument.
///
/// # Arguments
/// * `client` - A reference to the database client used to query the data.
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
/// ```rust
/// // Assuming you have an active `Client` instance
/// let client = get_database_client().await?;
/// let csv_path = "encounters.csv";
/// export_all_encounters_csv(&client, csv_path).await?;
/// ```
#[tracing::instrument(skip(client))]
async fn export_all_encounters_csv(client: &Client, fname: &str) -> Result<()> {
    trace!("Exporting all encounters from airplane_prox");

    let data = retrieve_all_encounters(client).await?;
    let len = data.len();

    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new().has_headers(true).from_writer(vec![]);

    // Insert data
    //
    data.into_iter().for_each(|rec| {
        assert!(rec.distance_slant_m >= rec.distance_hor_m);
        wtr.serialize(rec).unwrap();
    });

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
    fs::write(fname, data)?;
    trace!("Exported {} encounters", len);

    Ok(())
}

/// Exports all encounters as a Parquet file.
///
/// This function first generates a temporary CSV file from all encounters by invoking
/// the `export_all_encounters_csv` function. It then reads the CSV file into a DataFrame and
/// writes the DataFrame to a Parquet file in the specified location.
///
/// # Arguments
/// * `client` - A reference to the database client used to query data.
/// * `fname` - The output filename where the Parquet file will be saved.
///
/// # Returns
/// * `Result<()>` - Returns an `Ok` result on success, or an error if anything goes wrong.
///
/// # Process
/// 1. Creates a temporary CSV file using `tempfile`.
/// 2. Calls `export_all_encounters_csv` to populate the temporary CSV file with encounter data.
/// 3. Reads the CSV data into a DataFrame using Polars utilities.
/// 4. Writes the DataFrame data into a Parquet file format.
///
/// # Errors
/// This function may return errors in the following cases:
/// * Failure to create the temporary file.
/// * Issues with reading or writing CSV and Parquet files.
/// * Database query errors while retrieving the encounters data.
///
/// # Examples
/// ```rust
/// // Assuming you have a valid `Client` instance
/// let client = get_client().await?;
/// let fname = "output.parquet";
/// export_all_encounters_parquet(&client, fname).await?;
/// ```
///
#[tracing::instrument(skip(client))]
async fn export_all_encounters_parquet(client: &Client, fname: &str) -> Result<()> {
    let csv = Builder::new().suffix(".csv").tempfile()?;
    let tmpname = csv.path().to_string_lossy().to_string();
    trace!("Creating and saving CSV into {tmpname}");

    // Generate the csv file as `tmpname`
    //
    export_all_encounters_csv(client, &tmpname).await?;

    trace!("Writing {fname} as parquet.");

    // nh = no header line (default = false which means has header line).
    //
    let header = true;

    let opts = CsvParseOptions::default().with_try_parse_dates(true);
    let mut df = polars::prelude::CsvReadOptions::default()
        .with_has_header(header)
        .with_parse_options(opts)
        .try_into_reader_with_file_path(Some(tmpname.into()))?
        .finish()?;

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
/// * `dbh` - A reference to the database client used to query data.
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
#[tracing::instrument(skip(dbh))]
async fn export_all_encounters_summary_csv(dbh: &Client, fname: &str) -> eyre::Result<()> {
    // Create a temp file with all min distances
    //
    let data = retrieve_summary_encounters(dbh).await?;
    let len = data.len();

    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new().has_headers(true).from_writer(vec![]);

    // Insert data
    //
    data.into_iter().for_each(|rec| {
        wtr.serialize(rec).unwrap();
    });

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
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
#[tracing::instrument(skip(ctx))]
pub async fn export_results(ctx: &Context, opts: &ExpDistOpts) -> eyre::Result<()> {
    let client = ctx.db().await;

    // Do we export as a csv the "encounters of the day"?
    //
    match &opts.output {
        Some(fname) => {
            if opts.summary {
                export_all_encounters_summary_csv(&client, fname).await?
            } else {
                match opts.format {
                    Format::Csv => export_all_encounters_csv(&client, fname).await?,
                    Format::Parquet => export_all_encounters_parquet(&client, fname).await?,
                    _ => {
                        return {
                            eprintln!("Unknown format specified.");
                            Err(Status::UnknownFormat(opts.format.to_string()).into())
                        }
                    }
                }
            };
        }
        None => {
            eprintln!("No output file specified.");
            return Err(Status::NoOutputDestination.into());
        }
    }
    drop(client);
    info!("Done.");
    Ok(())
}
