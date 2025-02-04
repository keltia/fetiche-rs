//! Flightaware has this weird notion of a position and/or destination and/or origin
//! and encode these as a single string.
//!
//! It is supposed to represent an ICAO name, a waypoint name or a precise geo location
//!
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, space1},
    combinator::map,
    number::complete::float,
    sequence::{pair, preceded, terminated},
    IResult,
};
use serde::Deserialize;

/// Represents a location in the Flightaware system, which can be either a tagged name
/// (such as an ICAO code, airport code, or waypoint) or a geographical position
/// with latitude and longitude coordinates.
///
/// # Variants
///
/// - `Tag(String)`:
///   A string representing a tagged location, typically an identifier like an ICAO code
///   or a waypoint name.
///
///   Example: `KMCO`
///
/// - `Position { lat: f32, lon: f32 }`:
///   Represents a geographical position with latitude (`lat`) and longitude (`lon`) values.
///
///   Example: `L 41.04194 -95.34611`
///
/// # Example Usage
///
/// Parsing a tagged name:
/// ```
/// use fetiche_formats::{parse_location, Location};
///
/// let input = "KMCO";
/// let (_, location) = parse_location(input).unwrap();
/// assert_eq!(location, Location::Tag("KMCO".to_string()));
/// ```
///
/// Parsing a position:
/// ```
/// use fetiche_formats::{parse_location, Location};
///
/// let input = "L 41.04194 -95.34611";
/// let (_, location) = parse_location(input).unwrap();
/// assert_eq!(location, Location::Position { lat: 41.04194, lon: -95.34611 });
/// ```
///
#[derive(Clone, Debug, Deserialize, strum::Display, PartialEq)]
pub enum Location {
    /// Can be ICAOString or a Waypoint
    Tag(String),
    /// e.g. "L 41.04194 -95.34611"
    Position { lat: f32, lon: f32 },
}

/// Parses an input string into a `Location` enum, determining whether it represents
/// a tagged name (e.g., an ICAO code or waypoint) or a geographical position
/// (latitude and longitude).
///
/// # Arguments
///
/// * `input` - A string slice that holds the input to be parsed.
///
/// # Returns
///
/// An `IResult` containing any remaining input and a `Location` object if the parsing is successful.
///
/// # Examples
///
/// Parsing a tagged name:
/// ```
/// use fetiche_formats::{parse_location, Location};
///
/// let input = "JFK";
/// let (_, location) = parse_location(input).unwrap();
/// assert_eq!(location, Location::Tag("JFK".to_string()));
/// ```
///
/// Parsing a position:
/// ```
/// use fetiche_formats::{parse_location, Location};
///
/// let input = "L 40.7128 -74.0060";
/// let (_, location) = parse_location(input).unwrap();
/// assert_eq!(location, Location::Position { lat: 40.7128, lon: -74.0060 });
/// ```
///
pub fn parse_location(input: &str) -> IResult<&str, Location> {
    alt((position, tagged_name))(input)
}

#[inline]
fn tagged_name(input: &str) -> IResult<&str, Location> {
    map(alphanumeric1, |s: &str| Location::Tag(s.to_string()))(input)
}

#[inline]
fn position(input: &str) -> IResult<&str, Location> {
    let pos = |(lat, lon): (f32, f32)| Location::Position { lat, lon };

    let p = preceded(
        terminated(tag("L"), space1),
        pair(terminated(float, space1), float),
    );
    map(p, pos)(input)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("L 0.0 0.0", Location::Position{ lat: 0.0, lon: 0.0 })]
    #[case("L -90.0 -180.0", Location::Position{ lat: -90.0, lon: -180.0 })]
    #[case("L 90.0 180.0", Location::Position{ lat: 90.0, lon: 180.0 })]
    #[case("L 51.509865 -0.118092", Location::Position{ lat: 51.509865, lon: -0.118092 })]
    #[case("L 35.6895 139.6917", Location::Position{ lat: 35.6895, lon: 139.6917 })]
    fn test_parse_position(#[case] input: &str, #[case] l: Location) {
        let (_, loc) = parse_location(input).unwrap();
        assert_eq!(l, loc);
    }

    #[rstest]
    #[case("KMCO", Location::Tag("KMCO".to_string()))]
    #[case("L 41.8 -6.7", Location::Position{ lat: 41.8, lon: - 6.7})]
    fn test_parse_location(#[case] input: &str, #[case] l: Location) {
        let (_, loc) = parse_location(input).unwrap();
        assert_eq!(l, loc);
    }

    #[rstest]
    #[case("JFK", Location::Tag("JFK".to_string()))]
    #[case("SFO", Location::Tag("SFO".to_string()))]
    #[case("ATL", Location::Tag("ATL".to_string()))]
    #[case("L 12.34 -56.78", Location::Position{ lat: 12.34, lon: -56.78 })]
    #[case("L -23.456 78.91011", Location::Position{ lat: -23.456, lon: 78.91011 })]
    #[case("L 0.123 -0.456", Location::Position{ lat: 0.123, lon: -0.456 })]
    #[case("EWR", Location::Tag("EWR".to_string()))]
    #[case("ORD", Location::Tag("ORD".to_string()))]
    #[case("L 33.9425 -118.408056", Location::Position{ lat: 33.9425, lon: -118.408056 })]
    #[case("L 40.7128 -74.0060", Location::Position{ lat: 40.7128, lon: -74.0060 })]
    fn test_parse_location_additional(#[case] input: &str, #[case] l: Location) {
        let (_, loc) = parse_location(input).unwrap();
        assert_eq!(l, loc);
    }
}
