use chrono::{DateTime, Utc};
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Display, Formatter};
use std::ops::Add;
use strum::{EnumString, VariantNames};

#[derive(Clone, Debug, Deserialize, EnumString, Serialize, PartialEq, VariantNames)]
#[serde(rename_all = "lowercase")]
pub enum Producer {
    Fetch(String, Filter),
    Read(String),
    Stream(String),
}

#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Middle {
    Copy,
    Convert(String),
    Tee(String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Consumer {
    Archive(String),
    Store(String, Freq),
    Save(String),
}

#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Freq {
    Daily,
    Hourly,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobText {
    /// Job name.
    pub name: String,
    /// Generator
    pub producer: Producer,
    /// Optional list of filters like `Tee` or `Save`.
    pub filters: Option<Vec<Middle>>,
    /// Output file name.
    pub output: Consumer,
}

impl JobText {
    pub fn parse(job_str: &str) -> Result<JobText> {
        let j: JobText = hcl::from_str(job_str)?;
        println!("--\n{:?}\n--", j);
        Ok(j)
    }
}

#[derive(Clone, Debug)]
pub enum Task {
    /// Producer task that generates or sources data
    Producer(Producer),
    /// Middle task that transforms or processes data
    Middle(Middle),
    /// Consumer task that consumes or stores the final data
    Consumer(Consumer),
}

fn main() -> Result<()> {
    let f1 = Filter::interval(Utc::now(), Utc::now().add(chrono::Duration::days(1)));
    let f2 = Filter::stream(Utc::now().timestamp_millis(), 3600, 10);
    let j1 = JobText {
        producer: Producer::Fetch("test_source".to_string(), f1),
        name: "Test Job JobText".to_string(),
        filters: Some(vec![Middle::Tee("test_tee".to_string()), Middle::Copy]),
        output: Consumer::Save("test_output.csv".to_string()),
    };
    let str = hcl::to_string(&j1)?;
    println!("--\n{str}--");


    let j2 = JobText {
        producer: Producer::Fetch("test_source".to_string(), f2),
        name: "Fetch Test JobText".to_string(),
        filters: Some(vec![Middle::Tee("test_tee".to_string()), Middle::Copy]),
        output: Consumer::Store("test_output.csv".to_string(), Freq::Daily),
    };
    let str = hcl::to_string(&j2)?;
    println!("--\n{str}--");

    let j3 = JobText {
        producer: Producer::Fetch("test_source".to_string(), Filter::None),
        name: "Fetch Test JobText".to_string(),
        filters: None,
        output: Consumer::Store("test_output.csv".to_string(), Freq::Daily),
    };

    let str = hcl::to_string(&j3)?;
    println!("--\n{str}--");

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parse_job_text_valid() {
        let input = r#"
name = "Test Job JobText"
producer = {
  fetch = [
    "test_source",
    {
          "begin" = "2025-05-08T00:00:00.000000Z"
          "end" = "2025-05-09T00:00:00.000000Z"
    }
  ]
}
filters = [
  {
    tee = "test_tee"
  },
  "copy"
]
output = {
  save = "test_output.csv"
}    "#;

        let parsed_job = JobText::parse(input);
        dbg!(&parsed_job);

        let now: DateTime<Utc> = Utc.with_ymd_and_hms(2025, 05, 8, 0, 0, 0).unwrap();
        let tomorrow = now.add(chrono::Duration::days(1));
        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.producer, Producer::Fetch("test_source".into(), Filter::Interval { begin: now, end: tomorrow }));
        assert_eq!(job.name, "Test Job JobText");
        assert!(job.filters.is_some());
        let filters = job.filters.unwrap();
        assert_eq!(filters.len(), 2);
        assert!(matches!(filters[0], Middle::Tee(ref val) if val == "test_tee"));
        assert!(matches!(filters[1], Middle::Copy));
        assert_eq!(job.output, Consumer::Save("test_output.csv".to_string()));
    }

    #[test]
    fn parse_job_text_with_no_filters() {
        let input = r#"
    name = "Job Without Filters"
    producer = {
        "read" = "source_name"
    }
    output = {
        save = "no_filters.csv"
    }
    "#;

        let parsed_job = JobText::parse(input);
        dbg!(&parsed_job);

        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.producer, Producer::Read("source_name".into()));
        assert_eq!(job.name, "Job Without Filters");
        assert!(job.filters.is_none());
        assert_eq!(job.output, Consumer::Save("no_filters.csv".into()));
    }

    #[test]
    fn parse_job_text_with_store_daily() {
        let input = r#"
    name = "Job Without Filters"
    producer = {
        "read" = "source_name"
    }
    filters = null
    output = {
        store = [
            "/tmp/somewhere",
            "daily"
        ]
    }
    "#;

        let parsed_job = JobText::parse(input);
        dbg!(&parsed_job);

        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.producer, Producer::Read("source_name".into()));
        assert_eq!(job.name, "Job Without Filters");
        assert!(job.filters.is_none());
        assert_eq!(job.output, Consumer::Store("/tmp/somewhere".into(), Freq::Daily));
    }


    #[test]
    fn parse_job_text_invalid_type() {
        let input = r#"
    producer = "invalid"
    name = "Invalid Job"
    source = "invalid_source"
    output = {
        save = "output.csv"
    }
    "#;

        let parsed_job = JobText::parse(input);

        assert!(parsed_job.is_err());
    }

    #[test]
    fn parse_job_text_invalid_format() {
        let input = r#"
    job_type = "fetch"
    name = "Missing Required Field"
    "#;

        let parsed_job = JobText::parse(input);

        assert!(parsed_job.is_err());
    }
}


/// Represents various filtering criteria that can be used to specify
/// particular subsets of data or time intervals.
///
/// - `Interval`: Specifies a time interval using a `begin` and `end` datetime.
/// - `Keyword`: Represents a key-value pair middle.
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
///
/// let begin = dateparser::parse("2023-10-01").unwrap();
/// let end = dateparser::parse("2023-10-02").unwrap();
/// let middle = Filter::Interval { begin, end };
/// ```
///
/// ## Creating a Keyword Filter
/// ```rust
///
/// let middle = Filter::keyword("icao24", "foobar");
/// ```
///
/// ## Creating a Duration Filter
/// ```rust
///
/// let middle = Filter::since(3600); // Filter for the past hour
/// ```
///
/// ## Creating a Stream Filter
/// ```rust
///
/// let middle = Filter::stream(5, 3600, 10); // Stream starting at 5s, lasting 1 hour with a 10s delay
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
    /// Interpret argument as a json encoded middle
    ///
    fn from(value: &str) -> Self {
        let filter: std::result::Result<Filter, serde_json::Error> = serde_json::from_str(value);
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
