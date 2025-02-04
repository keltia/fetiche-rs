//! Compiler for the Fetiche job language
//!
//! Description of the job & task language -- leveraging HCL support.  The advantage is that we
//! could switch to JSON or anything else.
//!
//! ```hcl
//! name = "Opensky"
//! type = "fetch"
//! source = opensky
//! output = "foo.csv"
//! ```
//!
//! ```hcl
//! name = "Opensky"
//! type = "fetch"
//! source = opensky
//! filters = []
//! output = "foo.csv"
//! ```

use std::collections::VecDeque;

use eyre::Result;
use fetiche_common::Container;
use fetiche_formats::Format;
use ractor::call;
use serde::{Deserialize, Serialize};
use strum::EnumString;
use tracing::trace;

use crate::actors::{QueueMsg, SourcesMsg};
use crate::{
    Consumer, Copy, Engine, Fetch, Job, JobState, Middle, Producer, Read, Save, Store, Stream, Tee,
};

/// Represents the type of job to be executed.
///
/// This enum captures the various job types supported by the Fetiche language,
/// which defines how data is processed or retrieved. The job type is
/// serialized and deserialized using lowercase strings (e.g., "fetch", "read").
///
/// # Variants
///
/// - `Fetch`: Fetches data from an external source.
/// - `Read`: Reads data from an existing file or resource.
/// - `Stream`: Streams data directly from an external source in real-time.
///
#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ProducerText {
    /// One-shot fetch a block of data.
    Fetch(String),
    /// Read a local file.
    Read(String),
    /// Long-running job, streaming.
    Stream(String),
}

/// Represents the various types of filters/middleware that can be applied to a job.
///
/// Filters define additional processing or transformation steps to be
/// performed on the data during a job's execution. Each filter is associated
/// with a particular action or target.
///
/// # Variants
///
/// - `Tee`: Duplicates the data stream to the specified target.
/// - `Split`: Splits the job output into multiple paths or files.
/// - `Save`: Saves intermediate results to the specified path.
///
/// # Fields
///
/// `String` - The target or path associated with the filter action.
///
#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
enum MiddleText {
    //// Conversion between formats.
    Convert(Format),
    /// Block by block copy.
    Copy,
    /// Duplicate the data in a given file.
    Tee(String),
}

/// Represents the various types of consumers for processing or saving job outputs.
///
/// Consumers are used to define how or where the output data from a job
/// will be handled or distributed.
///
/// # Variants
///
/// - `Archive`: Archives the job output to the specified location.
/// - `Save`: Saves the job output to the specified file path.
/// - `Store`: Splits the job output into multiple files in the specified directory.
///
/// # Fields
///
/// Each variant has an associated `String` value, representing the path
/// or target destination for the consumer action.
///
#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
enum ConsumerText {
    /// Archive multiple files in a single one.
    Archive(String),
    /// Save in a file.
    Save(String),
    /// Store files by frequency in the specified directory.
    Store(String, Freq),
}

/// Represents the frequency options for scheduling or specifying periodic tasks.
///
/// This enum defines two main frequencies:
/// - **Daily:** Indicates a once-per-day schedule.
/// - **Hourly:** Indicates a once-per-hour schedule.
///
/// # Examples
///
/// ```rust
/// use fetiche_engine::Freq;
///
/// let daily = Freq::Daily;
/// let hourly = Freq::Hourly;
///
/// match daily {
///     Freq::Daily => println!("Task runs daily"),
///     Freq::Hourly => println!("Task runs hourly"),
/// }
/// ```
///
#[derive(Clone, Debug, Default, Deserialize, EnumString, PartialEq, Serialize)]
pub enum Freq {
    #[default]
    Daily,
    Hourly,
}

#[derive(Debug, Deserialize, Serialize)]
struct JobText {
    /// Job name.
    pub name: String,
    /// Data generator
    pub producer: ProducerText,
    /// Optional list of filters like `Tee` or `Save`.
    pub filters: Option<Vec<MiddleText>>,
    /// Output file name.
    pub output: ConsumerText,
}

impl Engine {
    /// Our job structure is
    /// - one producer
    /// - zero or more filters
    /// - one consumer
    ///
    pub async fn parse(&mut self, job_str: &str) -> Result<Job> {
        let jt: JobText = hcl::from_str(job_str)?;
        trace!("{:?}", &jt);

        // Assign a new ID
        //
        let id = call!(self.queue, |port| QueueMsg::Allocate(port))?;

        // Retrieve the site's data from the Sources actor.
        //
        let producer = match jt.producer {
            ProducerText::Fetch(p) => {
                let name = jt.name.clone();
                let site = call!(self.sources, |port| SourcesMsg::Get(name, port))?;
                let mut f = Fetch::new(&p);
                f.site(site);
                Producer::Fetch(f.clone())
            }
            ProducerText::Stream(p) => {
                let name = jt.name.clone();
                let site = call!(self.sources, |port| SourcesMsg::Get(name, port))?;
                let mut s = Stream::new(&p);
                s.site(site);
                Producer::Stream(s.clone())
            }
            ProducerText::Read(p) => {
                let r = Read::new(&p);
                Producer::Read(r)
            }
        };
        let list = if let Some(filters) = &jt.filters {
            filters
                .iter()
                .map(|t| match t {
                    MiddleText::Convert(_) => {
                        unimplemented!()
                    }
                    MiddleText::Copy => {
                        let c = Copy::new();
                        Middle::Copy(c)
                    }
                    MiddleText::Tee(fname) => {
                        let tee = Tee::into(&fname);
                        Middle::Tee(tee)
                    }
                })
                .collect()
        } else {
            VecDeque::new()
        };
        let consumer = match jt.output {
            ConsumerText::Archive(_) => Consumer::Invalid,
            ConsumerText::Save(c) => {
                let f = Save::new(&c, Format::None, Container::Raw);
                Consumer::Save(f)
            }
            ConsumerText::Store(c, f) => {
                let s = Store::new(&c, id, f)?;
                Consumer::Store(s)
            }
        };

        // Job is now Ready as it is complete with task list.
        //
        let job = Job {
            id,
            producer,
            name: jt.name.clone(),
            state: JobState::Ready,
            filters: list,
            consumer,
            stats: None,
        };
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[tokio::test]
    async fn test_parse_fetch_job() -> Result<()> {
        let mut engine = Engine::new().await;
        let job_str = r#"
            name = "test_fetch"
            producer = fetch "data"
            output = save "output.csv"
        "#;

        let job = engine.parse(job_str).await?;

        assert_eq!(job.name, "test_fetch");
        assert!(matches!(job.producer, Producer::Fetch(_)));
        assert!(matches!(job.consumer, Consumer::Save(_)));
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_parse_stream_job() -> Result<()> {
        let mut engine = Engine::new().await;
        let job_str = r#"
            name = "test_stream"
            producer = stream "data"
            output = save "output.csv"
        "#;

        let job = engine.parse(job_str).await?;

        assert_eq!(job.name, "test_stream");
        assert!(matches!(job.producer, Producer::Stream(_)));
        assert!(matches!(job.consumer, Consumer::Save(_)));
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_parse_read_job() -> Result<()> {
        let mut engine = Engine::new().await;
        let job_str = r#"
            name = "test_read"
            producer = read "input.csv"
            output = save "output.csv"
        "#;

        let job = engine.parse(job_str).await?;

        assert_eq!(job.name, "test_read");
        assert!(matches!(job.producer, Producer::Read(_)));
        assert!(matches!(job.consumer, Consumer::Save(_)));
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_parse_job_with_filters() -> Result<()> {
        let mut engine = Engine::new().await;
        let job_str = r#"
            name = "test_filters"
            producer = fetch "data"
            filters = [
                copy,
                tee "copy.csv"
            ]
            output = save "output.csv"
        "#;

        let job = engine.parse(job_str).await?;

        assert_eq!(job.filters.len(), 2);
        assert!(matches!(job.filters[0], Middle::Copy(_)));
        assert!(matches!(job.filters[1], Middle::Tee(_)));
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_parse_invalid_hcl() -> Result<()> {
        let mut engine = Engine::new().await;
        let job_str = r#"
            invalid hcl syntax
        "#;

        let result = engine.parse(job_str).await;
        assert!(result.is_err());
        Ok(())
    }
}
