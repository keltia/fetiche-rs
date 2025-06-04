//! Rotation duration parsing benchmark.
//!
//!

use nom::{
    character::complete::{i8, one_of},
    combinator::map_res,
    IResult,
    Parser,
};
use std::num::ParseIntError;

pub fn parse_rotation(input: &str) -> IResult<&str, u32> {
    let into_seconds = |(n, tag): (std::primitive::i8, char)| -> Result<u32, ParseIntError> {
        let res = match tag {
            's' => n as u32,
            'm' => (n as u32) * 60,
            'h' => (n as u32) * 3_600,
            'd' => (n as u32) * 3_600 * 24,
            _ => n as u32,
        };
        Ok(res)
    };
    map_res((i8, one_of("smhd")), into_seconds).parse(input)
}

fn main() -> eyre::Result<()> {
    let input = ["32s", "60s", "1h", "24h", "1d"];

    input.iter().for_each(|&input| {
        let (_, res) = parse_rotation(input).unwrap();
        println!("{} -> {:?}", input, res);
    });
    Ok(())
}