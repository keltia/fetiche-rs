//! sub-module to manage date (maybe geo ones in the future) filters
//!
//! A Filter is either a set of begin/end time points, a duration, a keyword/value couple or nothing.
//! This is used to pass arguments to sources but maybe be extended in the future.  This is different
//! from an argument or a set of arguments.
//!
//! FIXME It might be useful to simplify all this, maybe at some point a nom-based parser?  We have
//!       to define a syntax first.
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Display, Formatter};

/// Represents various filtering criteria that can be used to specify
/// particular subsets of data or time intervals.
///
/// - `Interval`: Specifies a time interval using a `begin` and `end` datetime.
/// - `Keyword`: Represents a key-value pair filter.
/// - `Duration`: Specifies a length of time in seconds. Negative values indicate
///               a period in the past.
/// - `Altitude`: Defines altitude-based filters with a `duration`, `min`, and `max` altitude.
/// - `Stream`: Represents streaming parameters such as start time (`from`),
///             `duration`, and `delay` between calls.
/// - `None`: Default variant for no filtering.
///
/// The `Filter` enum can be serialized and is compatible with JSON.
///
/// # Examples
///
/// ## Creating an Interval Filter
/// ```rust
/// use chrono::{Utc, TimeZone};
/// use fetiche_sources::Filter;
///
/// let begin = Utc.with_ymd_and_hms(2023, 10, 1, 0, 0, 0);
/// let end = Utc.with_ymd_and_hms(2023, 10, 10, 0, 0, 0);
/// let filter = Filter::interval(begin, end);
/// ```
///
/// ## Creating a Keyword Filter
/// ```rust
/// use fetiche_sources::Filter;
///
/// let filter = Filter::keyword("icao24", "foobar");
/// ```
///
/// ## Creating a Duration Filter
/// ```rust
/// use fetiche_sources::Filter;
///
/// let filter = Filter::since(3600); // Filter for the past hour
/// ```
///
/// ## Creating a Stream Filter
/// ```rust
/// use fetiche_sources::Filter;
///
/// let filter = Filter::stream(5, 3600, 10); // Stream starting at 5s, lasting 1 hour with a 10s delay
/// ```
///
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Filter {
    /// Date-based interval as "%Y-%m-%d %H:%M:%S"
    Interval {
        begin: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    /// Special parameter with name=value
    Keyword { name: String, value: String },
    /// Duration as length of time in seconds (can be negative to go in the past for N seconds)
    Duration(i32),
    /// Altitude is for min and max altitude you want drone data for (`AvionixCube`).
    Altitude {
        duration: u32,
        min: u32,
        max: u32,
    },
    /// Special interval for stream: do we go back slightly in time?  For how long?  Do we have a
    /// delay between calls?
    Stream {
        from: i64,
        duration: u32,
        delay: u32,
    },
    #[default]
    None,
}

impl Filter {
    /// from two time points
    ///
    pub fn interval(begin: DateTime<Utc>, end: DateTime<Utc>) -> Self {
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
    pub fn stream(from: i64, duration: u32, delay: u32) -> Self {
        Filter::Stream {
            from,
            duration,
            delay,
        }
    }
}

impl Display for Filter {
    /// We want the formatting to ignore the `Interval` vs `None`, it is easier to pass data around
    /// BTW this gives us `to_string()` as well.
    ///
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug, Serialize)]
        struct Minimal {
            begin: DateTime<Utc>,
            end: DateTime<Utc>,
        }

        #[derive(Debug, Serialize)]
        struct Keyword {
            name: String,
            value: String,
        }

        #[derive(Debug, Serialize)]
        struct Stream {
            from: i64,
            duration: u32,
            delay: u32,
        }

        #[derive(Debug, Serialize)]
        struct Altitude {
            duration: u32,
            min: u32,
            max: u32,
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
            Filter::Altitude { duration, min, max } => {
                let m = Altitude {
                    duration: *duration,
                    min: *min,
                    max: *max,
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
            Filter::Stream {
                from,
                duration,
                delay,
            } => {
                let s = Stream {
                    from: *from,
                    duration: *duration,
                    delay: *delay,
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
                Filter::Duration(_)
                | Filter::Interval { .. }
                | Filter::Keyword { .. }
                | Filter::Stream { .. } => f,
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
    use eyre::Result;
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

        let begin = dateparser::parse(&begin);
        assert!(begin.is_ok());
        let end = dateparser::parse(&end);
        assert!(end.is_ok());

        let f = Filter::interval(begin.unwrap(), end.unwrap());
        assert_ne!(Filter::None, f);
        println!("{}", json!(f));
        Ok(())
    }

    #[test]
    fn test_filter_interval_to_string() {
        let begin = "2022-11-11 12:34:56 UTC";
        let end = "2022-11-30 12:34:56 UTC";

        let begin = dateparser::parse(&begin);
        assert!(begin.is_ok());
        let end = dateparser::parse(&end);
        assert!(end.is_ok());

        let r = r##"{"begin":"2022-11-11T12:34:56Z","end":"2022-11-30T12:34:56Z"}"##;

        let f = Filter::interval(begin.unwrap(), end.unwrap());
        let s = f.to_string();
        assert_eq!(r, &s);

        let t: Filter = s.into();
        assert_eq!(f, t);
    }
}
