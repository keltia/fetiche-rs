//! sub-module to manage date (maybe geo ones in the future) filters
//!
//! A Filter is either a set of begin/end time points, a duration, a keyword/value couple or nothing.
//! This is used to pass arguments to sources but maybe be extended in the future.  This is different
//! from an argument or a set of arguments.
//!
//! XXX It might be useful to simplify all this, maybe at some point a nom-based parser?  We have
//!     to define a syntax first.
//!

use std::fmt::{Display, Formatter};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// If we specify -B/-E or --today, we need to pass these below
///
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Filter {
    /// Date-based interval as "%Y-%m-%d %H:%M:%S"
    Interval {
        begin: NaiveDateTime,
        end: NaiveDateTime,
    },
    /// Special parameter with name=value
    Keyword { name: String, value: String },
    /// Duration as length of time in seconds (can be negative to go in the past for N seconds)
    Duration(i32),
    /// Special interval for stream: do we go back slightly in time?  For how long?
    Stream { from: i32, duration: i32 },
    #[default]
    None,
}

impl Filter {
    /// from two time points
    ///
    pub fn interval(begin: NaiveDateTime, end: NaiveDateTime) -> Self {
        Filter::Interval { begin, end }
    }

    /// From a period of time
    ///
    pub fn since(d: i32) -> Self {
        Filter::Duration(d)
    }

    /// From a keyword
    ///
    pub fn keyword(name: &str, value: &str) -> Self {
        Filter::Keyword {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    /// For a stream
    ///
    pub fn stream(from: i32, duration: i32) -> Self {
        Filter::Stream { from, duration }
    }
}

impl Display for Filter {
    /// We want the formatting to ignore the `Interval` vs `None`, it is easier to pass data around
    ///
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug, Serialize)]
        struct Minimal {
            begin: NaiveDateTime,
            end: NaiveDateTime,
        }

        #[derive(Debug, Serialize)]
        struct Keyword {
            name: String,
            value: String,
        }

        #[derive(Debug, Serialize)]
        struct Stream {
            from: i32,
            duration: i32,
        }

        let s: String = match self {
            Filter::None => "{}".to_owned(),
            Filter::Interval { begin, end } => {
                let m = Minimal {
                    begin: *begin,
                    end: *end,
                };
                json!(m).to_string()
            }
            Filter::Duration(d) => json!(d).to_string(),
            Filter::Keyword { name, value } => {
                let k = Keyword {
                    name: name.to_string(),
                    value: value.to_string(),
                };
                json!(k).to_string()
            }
            Filter::Stream { from, duration } => {
                let s = Stream {
                    from: *from,
                    duration: *duration,
                };
                json!(s).to_string()
            }
        };
        write!(f, "{}", s)
    }
}

impl From<&str> for Filter {
    /// Interpret argument as a json encoded filter
    ///
    fn from(value: &str) -> Self {
        let filter: Result<Filter, serde_json::Error> = serde_json::from_str(value);
        match filter {
            Ok(f) => match f {
                Filter::Duration(_) | Filter::Interval { .. } | Filter::Keyword { .. } => f,
                _ => Filter::None,
            },
            _ => Filter::None,
        }
    }
}

impl From<String> for Filter {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_filter_new() {
        assert_eq!(Filter::None, Filter::default())
    }

    #[test]
    fn test_filter_keyword() {
        let filter = Filter::keyword("icao24", "foobar");
        let res = format!("{}", filter);

        assert_eq!("{\"name\":\"icao24\",\"value\":\"foobar\"}", res);
    }

    #[test]
    fn test_filter_keyword_from_str() {
        let filter: Filter = "{\"name\":\"icao24\",\"value\":\"foobar\"}".into();

        assert_eq!(
            Filter::Keyword {
                name: "icao24".to_string(),
                value: "foobar".to_string(),
            },
            filter
        );
    }

    #[rstest]
    #[case(3600, "3600")]
    #[case(- 60, "-60")]
    fn test_filter_duration_to_string(#[case] inb: i32, #[case] out: &str) {
        let filter = Filter::Duration(inb);
        let str = filter.to_string();

        assert_eq!(out, str)
    }

    #[test]
    fn test_filter_keyword_to_string() {
        let filter = Filter::keyword("icao24", "foobar");
        let str = filter.to_string();

        assert_eq!("{\"name\":\"icao24\",\"value\":\"foobar\"}", str);
    }

    #[test]
    fn test_filter_interval_new() -> Result<()> {
        let begin = "2022-11-11 12:34:56";
        let end = "2022-11-30 12:34:56";

        let begin = NaiveDateTime::parse_from_str(begin, "%Y-%m-%d %H:%M:%S");
        assert!(begin.is_ok());
        let end = NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S");
        assert!(end.is_ok());

        let f = Filter::interval(begin.unwrap(), end.unwrap());
        assert_ne!(Filter::None, f);
        println!("{}", json!(f));
        Ok(())
    }

    #[test]
    fn test_filter_interval_to_string() {
        let begin = "2022-11-11 12:34:56";
        let end = "2022-11-30 12:34:56";

        let begin = NaiveDateTime::parse_from_str(begin, "%Y-%m-%d %H:%M:%S");
        assert!(begin.is_ok());
        let end = NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S");
        assert!(end.is_ok());

        let r = r##"{"begin":"2022-11-11T12:34:56","end":"2022-11-30T12:34:56"}"##;

        let f = Filter::interval(begin.unwrap(), end.unwrap());
        let s = f.to_string();
        assert_eq!(r, &s);

        let t: Filter = s.into();
        assert_eq!(f, t);
    }
}
