//! Rotation duration parsing benchmark.
//!
//!
use divan::{black_box, Bencher};
use nom::combinator::map_res;
use nom::Parser;

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_nom(bench: Bencher) {
    use nom::{
        character::complete::{i8, one_of},
        IResult,
    };

    #[inline]
    fn parse_rotation(input: &str) -> IResult<&str, u32> {
        let into_seconds = |(n, tag): (std::primitive::i8, char)| ->
        Result<u32, std::num::ParseIntError> {
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

    bench.bench_local(move || black_box(parse_rotation("24h")));
}

#[divan::bench]
fn bench_jiff(bench: Bencher) {
    use jiff::{Error, Span, SpanRelativeTo, Unit};

    #[inline]
    fn parse_rotation_jiff(date: &str) -> Result<f64, Error> {
        let v: Span = date.parse()?;
        v.total(Unit::Second)
    }

    bench.bench_local(move || black_box(parse_rotation_jiff("24h")));
}