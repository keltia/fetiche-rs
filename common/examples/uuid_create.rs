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
use jiff::{
    civil::DateTime,
    tz::TimeZone,
    {Span, SpanRelativeTo, Unit},
};
use uuid::{ContextV7, Uuid};

/// Command line options for the journey ID generator
///
#[derive(Parser, Debug)]
struct Opts {
    /// Time threshold in seconds to determine when to start a new journey
    #[clap(short = 't', long, default_value = "5m")]
    threshold: String,
    #[clap(short = 'o', long, default_value = "output.csv")]
    output: String,
    /// Input CSV file path containing trajectory points
    fname: String,
}

/// Parses a time duration string into a total number of seconds.
///
/// # Arguments
///
/// * `s` - A string representing a time duration (e.g., "30s", "1m", "2h")
///
/// # Returns
///
/// Returns a Result containing:
/// - `Ok(f64)` - The total number of seconds if parsing succeeds
/// - `Err` - If the string cannot be parsed or conversion fails
///
/// # Examples
///
/// ```
/// use uuid_create::parse_threshold;
///
/// assert_eq!(parse_threshold("30s").unwrap(), 30.0);
/// assert_eq!(parse_threshold("1m").unwrap(), 60.0);
/// assert_eq!(parse_threshold("1h").unwrap(), 3600.0);
/// ```
fn parse_threshold(s: &str) -> Result<f64> {
    let span = s.parse::<Span>()?;
    span.total((Unit::Second, SpanRelativeTo::days_are_24_hours()))
        .map_err(|e| eyre::eyre!(e))
}

fn process_one(
    previous_record: StringRecord,
    current_record: StringRecord,
    threshold: f64,
    uuid_ctx: &ContextV7,
) -> Result<StringRecord> {
    let marker = SpanRelativeTo::days_are_24_hours();

    // Extract data from records
    let current_journey_id = &current_record[0];
    let current_ident = &current_record[1];
    let current_tm_str = &current_record[2];
    let current_tm: DateTime = current_tm_str.parse()?;

    let previous_journey_id = &previous_record[0];
    let previous_ident = &previous_record[1];
    let previous_tm: DateTime = previous_record[2].parse()?;

    // Determine the journey ID for the new record
    //
    let new_journey_id = if current_ident != previous_ident {
        // Different drone, so it's a different journey by definition
        //
        current_journey_id.to_string()
    } else {
        // Same drone, check time difference
        //
        let diff = (current_tm - previous_tm).total((Unit::Second, marker))?;

        if diff > threshold {
            // Time difference is over the threshold, start a new journey
            //
            let unix_tm = current_tm.to_zoned(TimeZone::UTC)?.timestamp().as_second();
            let ts = uuid::Timestamp::from_unix(uuid_ctx, unix_tm as u64, 0);
            Uuid::new_v7(ts).to_string()
        } else {
            // Still the same journey
            //
            previous_journey_id.to_string()
        }
    };

    Ok(StringRecord::from(vec![
        new_journey_id,
        current_ident.to_string(),
        current_tm_str.to_string(),
    ]))
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
    let threshold = parse_threshold(&opts.threshold)?;

    // input/output
    //
    let mut rdr = ReaderBuilder::new()
        .delimiter(b',')
        .from_path(&opts.fname)?;
    let mut wrt = WriterBuilder::new()
        .delimiter(b',')
        .from_path(&opts.output)?;

    // Write the header to the output file immediately
    //
    wrt.write_record(&["uuid", "ident", "timestamp"])?;

    let mut records = rdr.records();
    let ctx = ContextV7::new();

    // Process the first record separately to establish the initial "previous" state
    //
    if let Some(first_result) = records.next() {
        let mut prev = first_result?;
        wrt.write_record(&prev)?;

        // Loop through the rest of the records, writing each result as we go
        for result in records {
            let record = result?;
            let new_record = process_one(prev, record, threshold, &ctx)?;
            wrt.write_record(&new_record)?;

            // The new record becomes the "previous" one for the next iteration
            //
            prev = new_record;
        }
    }

    wrt.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_threshold_seconds() {
        assert_eq!(parse_threshold("30s").unwrap(), 30.0);
        assert_eq!(parse_threshold("45s").unwrap(), 45.0);
    }

    #[test]
    fn test_parse_threshold_minutes() {
        assert_eq!(parse_threshold("1m").unwrap(), 60.0);
        assert_eq!(parse_threshold("2m").unwrap(), 120.0);
    }

    #[test]
    fn test_parse_threshold_hours() {
        assert_eq!(parse_threshold("1h").unwrap(), 3600.0);
    }

    #[test]
    fn test_parse_threshold_invalid() {
        assert!(parse_threshold("invalid").is_err());
        assert!(parse_threshold("").is_err());
    }

    #[test]
    fn test_process_one_same_drone_under_threshold() {
        let prev = StringRecord::from(vec!["journey1", "drone1", "2024-01-01T10:00:00"]);
        let curr = StringRecord::from(vec!["journey1", "drone1", "2024-01-01T10:00:15"]);
        let result = process_one(prev.clone(), curr, 30.0).unwrap();
        assert_eq!(result[0], prev[0]); // Should keep same journey ID
    }

    #[test]
    fn test_process_one_same_drone_over_threshold() {
        let prev = StringRecord::from(vec!["journey1", "drone1", "2024-01-01T10:00:00"]);
        let curr = StringRecord::from(vec!["journey1", "drone1", "2024-01-01T10:01:00"]);
        let result = process_one(prev, curr, 30.0).unwrap();
        assert_ne!(result[0], "journey1".to_string()); // Should generate new UUID
        assert!(Uuid::parse_str(&result[0]).is_ok()); // Should be valid UUID
    }

    #[test]
    fn test_process_one_different_drone() {
        let prev = StringRecord::from(vec!["journey1", "drone1", "2024-01-01T10:00:00"]);
        let curr = StringRecord::from(vec!["journey2", "drone2", "2024-01-01T10:00:15"]);
        let result = process_one(prev, curr.clone(), 30.0).unwrap();
        assert_eq!(result[0], curr[0]); // Should keep original journey ID
    }
}