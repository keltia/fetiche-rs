//! Compiler for the Fetiche job language
//!
//! Description of the job & task language
//!
//! >NOTE: Highly subject to changes
//!
//! ```text
//! job "Fetch Opensky data" is
//!     fetch "opensky"
//!     into Cat21
//!     output "aeroscope.csv"
//! end
//! ```

use nom::{
    bytes::complete::{tag, take_until},
    character::complete::{alphanumeric1, space1},
    combinator::map,
    sequence::{delimited, preceded, tuple},
    IResult,
};

use crate::Cmds;

/// Delimiter for strings, only " supported
const DELIM: &str = "\"";

/// Parse a string surrounded by "double quotes"
///
fn parse_string(input: &str) -> IResult<&str, &str> {
    delimited(tag(DELIM), take_until(DELIM), tag(DELIM))(input)
}

/// Parse a keyword (i.e. "message")
///
fn parse_keyword(input: &str) -> IResult<&str, &str> {
    alphanumeric1(input)
}

/// Parse a job definition, currently <command>\s+"<string>"
///
pub fn parse_job(input: &str) -> IResult<&str, (Cmds, String)> {
    let m = |(_k, m): (&str, &str)| (Cmds::Message, m.to_string());

    let line = tuple((parse_keyword, preceded(space1, parse_string)));
    map(line, m)(input)
}

#[cfg(test)]
mod tests {
    use crate::Cmds::Message;

    use super::*;

    #[test]
    fn test_parse_string_ascii() {
        env_logger::init();

        let s = r##""foo""##;
        let r = parse_string(s);

        assert!(r.is_ok());
        let (i, r) = r.unwrap();
        println!("r={r}");
        assert!(i.is_empty());
        assert_eq!("foo", r);
    }

    #[test]
    fn test_parse_string_utf8() {
        let s = r##""ねこ""##;
        let r = parse_string(s);

        assert!(r.is_ok());
        let (i, r) = r.unwrap();
        println!("r={r}");
        assert!(i.is_empty());
        assert_eq!("ねこ", r);
    }

    #[test]
    fn test_parse_job() {
        let s = "message \"foobar\"";

        let r = parse_job(s);
        dbg!(&r);
        assert!(r.is_ok());
        let (i, r) = r.unwrap();
        assert_eq!(Message, r.0);
        assert_eq!("foobar", r.1);
    }
}
