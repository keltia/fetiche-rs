//! Rotation duration parsing benchmark.
//!
//!
use divan::{black_box, Bencher};

fn main() {
    divan::main();
}

#[divan::bench]
fn bench_nom(bench: Bencher) {
    use nom::{
        character::complete::{i8, one_of},
        combinator::map,
        sequence::tuple,
        IResult,
    };

    #[inline]
    fn parse_rotation(input: &str) -> IResult<&str, u32> {
        let into_s = |(n, tag): (std::primitive::i8, char)| match tag {
            's' => n as u32,
            'm' => (n as u32) * 60,
            'h' => (n as u32) * 3_600,
            _ => n as u32,
        };
        let r = tuple((i8, one_of("smh")));
        map(r, into_s)(input)
    }

    bench.bench_local(move || black_box(parse_rotation("24h")));
}

#[divan::bench]
fn bench_jiff(bench: Bencher) {
    use jiff::{Error, Span, SpanRelativeTo, Unit};

    #[inline]
    fn jiff_timestamp(date: &str) -> Result<f64, Error> {
        let v: Span = date.parse()?;
        v.total(Unit::Second)
    }

    bench.bench_local(move || black_box(jiff_timestamp("24h")));
}