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
    #[clap(short = 't', long, default_value = "30")]
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

/// Processes a single trajectory point record and determines if it belongs to a new journey
/// based on time differences with the previous point.
///
/// # Arguments
///
/// * `acc` - The previous record containing journey_id, drone_id and timestamp
/// * `record` - The current record to process containing journey_id, drone_id and timestamp
/// * `threshold` - Time threshold in seconds to determine when to start a new journey
///
/// # Returns
///
/// Returns a Result containing:
/// - `Ok(StringRecord)` - New record with potentially updated journey ID
/// - `Err` - If timestamp parsing or calculations fail
///
/// # Examples
///
/// ```
/// use csv::StringRecord;
///
/// let prev = StringRecord::from(vec!["journey1", "drone1", "2024-01-01T10:00:00"]);
/// let curr = StringRecord::from(vec!["journey1", "drone1", "2024-01-01T10:00:15"]);
/// let result = process_one(prev, curr, 30.0).unwrap();
/// ```
///
fn process_one(acc: StringRecord, record: StringRecord, threshold: f64) -> Result<StringRecord> {
    let marker = SpanRelativeTo::days_are_24_hours();

    // Current record
    //
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
            // Starting a new journey with an uuid, use the record timestamp as base for it
            //
            let ctx = ContextV7::new();
            let unx = curr_tm
                .to_zoned(TimeZone::UTC)
                .unwrap()
                .timestamp()
                .as_second();
            let base_ts = uuid::Timestamp::from_unix(ctx, unx as u64, 0);
            let uuid = Uuid::new_v7(base_ts);
            StringRecord::from(vec![uuid.to_string(), ident, tm])
        } else {
            // Same journey, just another point
            //
            StringRecord::from(vec![prev_journey, ident, tm])
        }
    } else {
        // Different drone now
        //
        StringRecord::from(vec![journey, ident, tm])
    };
    Ok(new)
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

    let mut rdr = ReaderBuilder::new()
        .delimiter(b',')
        .from_path(opts.fname.clone())?;
    let mut data = rdr.records();

    let prev = data.next().unwrap()?;

    // Initialise with the header
    //
    let mut result = vec![StringRecord::from(vec!["uuid", "ident", "timestamp"])];

    // Now process the whole file, using `fold` to keep the previous record in the accumulator
    //
    let _ = data.fold(prev.clone(), |acc, r| {
        let new = process_one(acc, r.unwrap(), threshold).unwrap();

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
    let mut wrt = WriterBuilder::new().delimiter(b',').from_path(output)?;

    for record in result {
        wrt.write_record(&record)?;
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
