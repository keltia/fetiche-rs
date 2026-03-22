//! tz module for timezone handling
//!
use jiff::{tz::TimeZone, Timestamp};
use tzf_rs::DefaultFinder;

/// Represents timezone information for a geographical location.
///
/// This structure contains the IANA timezone name and the UTC offset in seconds
/// for a specific location at a given point in time. It is typically returned by
/// the `find_tz` function after querying timezone data based on coordinates.
///
/// # Fields
///
/// * `tzname` - The IANA timezone identifier (e.g., "America/New_York", "Europe/London")
/// * `offset` - The UTC offset in seconds at the queried time (positive for east of UTC, negative for west)
///
#[derive(Debug)]
pub struct TzEntry {
    /// Timezone name
    pub tzname: String,
    /// UTC offset in seconds
    pub offset: i32,
}

/// Finds timezone information for a given geographical location.
///
/// This function takes latitude and longitude coordinates and returns the timezone
/// name and UTC offset for that location. It uses the `tzf-rs` crate to determine
/// the timezone name based on geographical boundaries, and the `jiff` crate to
/// calculate the current UTC offset for that timezone.
///
/// # Arguments
///
/// * `lat` - The latitude coordinate in decimal degrees (range: -90.0 to 90.0)
/// * `lon` - The longitude coordinate in decimal degrees (range: -180.0 to 180.0)
///
/// # Returns
///
/// Returns a `Result` containing a `TzEntry` with:
/// * `tzname` - The IANA timezone name (e.g., "America/New_York", "Europe/Paris")
/// * `offset` - The UTC offset in seconds at the current time
///
/// # Errors
///
/// This function will return an error if:
/// * The timezone name cannot be found for the given coordinates
/// * The timezone name is invalid or not recognized by the `jiff` crate
///
/// # Examples
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Find timezone for New York City
/// # use fetiche_common::find_tz;
///
/// let tz = find_tz(40.7128, -74.0060)?;
/// println!("Timezone: {}, Offset: {}s", tz.tzname, tz.offset);
/// # Ok(())
/// # }
/// ```
///
#[tracing::instrument]
pub fn find_tz(lat: f64, lon: f64) -> eyre::Result<TzEntry> {
    let tzname = DefaultFinder::new().get_tz_name(lon, lat).to_string();
    let tz = TimeZone::get(&tzname)?;

    // We need a random timestamp
    //
    let tm = Timestamp::now();
    let offset = tz.to_offset(tm).seconds();

    Ok(TzEntry { tzname, offset })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(48.8566, 2.3522, "Europe/Paris")] // Paris
    #[case(40.7128, -74.0060, "America/New_York")] // New York
    #[case(35.6762, 139.6503, "Asia/Tokyo")] // Tokyo
    #[case(51.5074, -0.1278, "Europe/London")] // London
    #[case(-33.8688, 151.2093, "Australia/Sydney")] // Sydney
    fn test_find_tz_valid_coordinates(
        #[case] lat: f64,
        #[case] lon: f64,
        #[case] expected_tz: &str,
    ) {
        let result = find_tz(lat, lon);
        assert!(result.is_ok());
        let tz_entry = result.unwrap();
        assert_eq!(tz_entry.tzname, expected_tz);
        // Verify offset is within reasonable bounds (-12 to +14 hours in seconds)
        assert!(tz_entry.offset >= -43200 && tz_entry.offset <= 50400);
    }

    #[test]
    fn test_find_tz_invalid_coordinates() {
        // Test with coordinates that might cause issues
        let result = find_tz(91.0, 0.0); // Invalid latitude
        // Note: tzf_rs may handle invalid coordinates differently,
        // this test ensures the function doesn't panic
        let _ = result;
    }
}
