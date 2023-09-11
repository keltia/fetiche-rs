//! Compiler for the Fetiche job language
//!
//! ```text
//! ## Description of the job & task language
//!
//! >NOTE: Highly subject to changes
//!
//! ```text
//! job "Fetch Opensky data" is
//!     schedule every(5mn) | at(DATE)[,at(DATE)]*  // ?
//!     
//!     message "Beginning"
//!
//!     fetch("opensky")
//!     
//!     message "Transform into Cat21"
//!     
//!     into(Cat21)
//!     
//!     output("aeroscope.csv")
//! end
//! ```
//!

//use nom::{complete::tag, IResult};

// fn parse_message(input: &str) -> IResult<&str, &str> {}
//
// struct Schedule {}
//
// fn parse_schedule(input: &str) -> IResult<&str, Schedule> {}
//
// fn parse_fetch(input: &str) -> IResult<&str, &str> {}

use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{alphanumeric1, space1};
use nom::combinator::map;
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;

use crate::Cmds;

fn parse_string(input: &str) -> IResult<&str, &str> {
    delimited(tag("\""), take_until("\""), tag("\""))(input)
}

fn parse_keyword(input: &str) -> IResult<&str, &str> {
    alphanumeric1(input)
}

/// Parse a string surrounded by double quotes
///
pub fn parse_job(input: &str) -> IResult<&str, Cmds> {
    let m = |(k, m): (&str, &str)| Cmds::Message;

    let line = tuple((parse_keyword, preceded(space1, parse_string)));
    map(line, m)(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string() {
        env_logger::init();

        let s = r##""foo""##;
        let r = parse_string(s);

        assert!(r.is_ok());
        let (i, r) = r.unwrap();
        println!("r={r}");
        assert!(i.is_empty());
        assert_eq!("foo", r);
    }
}
