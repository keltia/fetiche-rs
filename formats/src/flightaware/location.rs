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

#[derive(Clone, Debug, Deserialize, strum::Display, PartialEq)]
pub enum Location {
    /// Can be ICAOString or a Waypoint
    Tag(String),
    /// e.g. "L 41.04194 -95.34611"
    Position { lat: f32, lon: f32 },
}

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
    #[case("KMCO", Location::Tag("KMCO".to_string()))]
    #[case("L 41.8 -6.7", Location::Position { lat: 41.8, lon: - 6.7})]
    fn test_parse_location(#[case] input: &str, #[case] l: Location) {
        let (_, loc) = parse_location(input).unwrap();
        assert_eq!(l, loc);
    }
}
