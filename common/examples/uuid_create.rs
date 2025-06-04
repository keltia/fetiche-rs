//! Generate unique journey IDs for drone trajectory data based on time thresholds.
//!
//! This example demonstrates how to:
//! - Read trajectory points from a CSV file
//! - Group points into journeys based on time differences
//! - Generate UUIDv7 identifiers for new journeys
//! - Output the results to a new CSV file
//!
//! # Input Format
//!
//! The input CSV file should contain trajectory points with the following columns:
//! - journey_id: Current journey identifier
//! - drone_id: Identifier of the drone
//! - timestamp: Time of the trajectory point
//!
//! # Example Usage
//!
//! ```bash
//! uuid_create --threshold 30 input.csv
//! ```
//!
//! # Notes
//!
//! - Uses UUIDv7 for timestamp-based unique journey identifiers
//! - Points from the same drone separated by more than the threshold start a new journey
//! - Output is written to 'output.csv' with columns: uuid, ident, timestamp
//!
use clap::Parser;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use eyre::Result;
use jiff::civil::DateTime;
use jiff::{SpanRelativeTo, Unit};
use uuid::{NoContext, Uuid};

/// Command line options for the journey ID generator
///
#[derive(Parser, Debug)]
struct Opts {
    /// Time threshold in seconds to determine when to start a new journey
    #[clap(short = 't', long, default_value = "30")]
    threshold: usize,
    #[clap(short = 'o', long, default_value = "output.csv")]
    output: String,
    /// Input CSV file path containing trajectory points
    fname: String,
}

/// Takes a CSV file containing trajectory points and assigns journey IDs based on time thresholds.
///
/// # Arguments
///
/// * `threshold` - Time threshold in seconds to determine when to start a new journey
/// * `fname` - Input CSV file path containing trajectory points
///
/// # Errors
///
/// Returns an error if:
/// - The input file cannot be read
/// - The CSV parsing fails
/// - Writing to output.csv fails
///
/// # Returns
///
/// Returns `Ok(())` on successful processing and writing of the output file
/// 
fn main() -> Result<()> {
    let opts = Opts::parse();
    let marker = SpanRelativeTo::days_are_24_hours();

    let threshold = opts.threshold as f64;

    let mut rdr = ReaderBuilder::new().delimiter(b',').from_path(opts.fname.clone())?;
    let mut data = rdr.records();

    let base_ts = uuid::Timestamp::now(NoContext);

    let prev = data.next().unwrap()?;

    // Initialize with the header
    //
    let mut result = vec![
        StringRecord::from(vec!["uuid", "ident", "timestamp"])
    ];

    // Now process the whole file, using `fold` to keep the previous record in the accumulator
    //
    let _ = data.fold(prev.clone(), |acc, r| {
        // Current record
        //
        let record = r.unwrap();
        let journey = record[0].to_string();
        let ident = record[1].to_string();
        let tm = record[2].to_string();
        let curr_tm: DateTime = tm.parse().unwrap();

        // get previous state
        //
        let prev_journey = acc[0].to_string();
        let prev_ident = acc[1].to_string();
        let prev_tm: DateTime = acc[2].parse().unwrap();

        let new = if prev_ident == ident {
            // Time difference
            //
            let diff = curr_tm - prev_tm;
            let diff = diff.total((Unit::Second, marker)).unwrap();

            // Same drone
            //
            if diff > threshold {
                // Starting a new journey with an uuid
                //
                let uuid = Uuid::new_v7(base_ts);
                StringRecord::from(vec![
                    uuid.to_string(),
                    ident,
                    tm,
                ])
            } else {
                // Same journey, just another point
                //
                StringRecord::from(vec![
                    prev_journey,
                    ident,
                    tm,
                ])
            }
        } else {
            // Different drone now
            //
            StringRecord::from(vec![
                journey,
                ident,
                tm,
            ])
        };

        // Now store the update record
        //
        result.push(new.clone());

        // This becomes our previous state
        //
        new
    });

    // Now write everything out
    //
    let output = opts.output.clone();
    let mut wrt = WriterBuilder::new()
        .delimiter(b',')
        .from_path(output)?;

    for record in result {
        wrt.write_record(&record)?;
    }
    wrt.flush()?;
    Ok(())
}
