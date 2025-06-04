//! Compiler for the Fetiche job language
//!
//! Description of the job & task language -- leveraging HCL support.  The advantage is that we
//! could switch to JSON or anything else.
//!
//! See [this example](../../examples/parse.rs) for a simple usage example.
//!
use std::collections::VecDeque;

use eyre::Result;
use fetiche_common::Container;
use fetiche_formats::Format;
use ractor::call;
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};
use tracing::{debug, trace};

use crate::actors::{SchedulerMsg, SourcesMsg};
use crate::{
    Consumer, Copy, Engine, Fetch, Filter, Job, JobState, Middle, Producer, Read, Save, Store,
    Stream, Tee,
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
enum ProducerText {
    /// One-shot fetch a block of data.
    Fetch(String, Filter),
    /// Read a local file.
    Read(String),
    /// Long-running job, streaming.
    Stream(String, Filter),
}

/// Represents the various types of filters/middleware that can be applied to a job.
///
/// Filters define additional processing or transformation steps to be
/// performed on the data during a job's execution. Each middle is associated
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
/// `String` - The target or path associated with the middle action.
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
#[derive(
    Clone,
    Debug,
    Default,
    Deserialize,
    EnumString,
    PartialEq,
    Serialize,
    strum::Display,
    VariantNames,
)]
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
    /// Parses a job definition string in HCL format and creates a new `Job` instance.
    ///
    /// This method takes a job definition in HashiCorp Configuration Language (HCL) format
    /// and transforms it into a fully configured `Job` structure. The job definition includes:
    /// - Producer configuration (Fetch, Read, or Stream)
    /// - Optional middleware filters
    /// - Consumer configuration (Save or Store)
    ///
    /// # Arguments
    ///
    /// * `job_str` - A string slice containing the HCL-formatted job definition
    ///
    /// # Returns
    ///
    /// Returns a `Result<Job>` which is:
    /// - `Ok(Job)` - A fully configured Job instance ready for execution
    /// - `Err(e)` - An error if parsing fails or the job configuration is invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[tokio::main]
    /// # async fn main() -> eyre::Result<()> {
    /// # use fetiche_engine::Engine;
    /// # let engine = Engine::single().await?;
    /// let job_str = r#"
    ///     name = "example_job"
    ///     producer = {
    ///         "Fetch" = ["source", { "duration" = 3600 }]
    ///     }
    ///     output = {
    ///         "Save" = "output.csv"
    ///     }
    /// "#;
    /// let job = engine.parse(job_str).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    pub async fn parse(&mut self, job_str: &str) -> Result<Job> {
        let jt: JobText = hcl::from_str(job_str)?;
        trace!("{:?}", &jt);

        // Assign a new ID
        //
        let id = call!(self.scheduler, SchedulerMsg::Allocate)?;

        // Retrieve the site's data from the Sources actor.
        //
        let producer = match jt.producer {
            ProducerText::Fetch(p, args) => {
                let site = call!(self.sources, |port| SourcesMsg::Get(p, port))?;
                let mut f = Fetch::new(&jt.name);
                f.site(site?);
                f.with(args);
                Producer::Fetch(f.clone())
            }
            ProducerText::Stream(p, args) => {
                let site = call!(self.sources, |port| SourcesMsg::Get(p, port))?;
                let mut s = Stream::new(&jt.name);
                s.site(site?);
                s.with(args);
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
                        let tee = Tee::into(fname);
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
                let mut f = Save::new(&c, Format::None, Container::Raw);
                f.path(&c);
                Consumer::Save(f)
            }
            ConsumerText::Store(c, f) => {
                let s = Store::new(&c, id, f).await?;
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
            middle: list,
            consumer,
            stats: None,
        };
        debug!("Job: {:?}", job);
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_fetch_job() -> Result<()> {
        let mut engine = Engine::new().await;
        engine.ps();

        let job_str = r#"
            name = "test_fetch"
            producer = {
                "Fetch" = [
                    "lux-me",
                    {
                        "from" = 1746898373376
                        "duration" = 3600
                        "delay" = 10
                    }
                ]
            }
            output = {
                "Save" = "output.csv"
            }
        "#;

        let job = engine.parse(job_str).await?;

        assert_eq!(job.name, "test_fetch");
        assert!(matches!(job.producer, Producer::Fetch(_)));
        assert!(matches!(job.consumer, Consumer::Save(_)));

        engine.shutdown();
        Ok(())
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_stream_job() -> Result<()> {
        let mut engine = Engine::new().await;
        engine.ps();

        let job_str = r#"
            name = "test_stream"
            producer = {
                "Stream" = [
                    "lux-me",
                    {
                        "from" = 1746898373376
                        "duration" = 3600
                        "delay" = 10
                    }
                ]
            }
            output = {
                "Save" = "output.csv"
            }
        "#;

        let job = engine.parse(job_str).await?;

        assert_eq!(job.name, "test_stream");
        assert!(matches!(job.producer, Producer::Stream(_)));
        assert!(matches!(job.consumer, Consumer::Save(_)));
        engine.shutdown();
        Ok(())
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_read_job() -> Result<()> {
        let mut engine = Engine::new().await;
        engine.ps();

        let job_str = r#"
            name = "test_read"
            producer = {
                "Read" = "input.csv"
            }
            output = {
                "Save" = "output.csv"
            }
        "#;

        let job = engine.parse(job_str).await?;

        assert_eq!(job.name, "test_read");
        assert!(matches!(job.producer, Producer::Read(_)));
        assert!(matches!(job.consumer, Consumer::Save(_)));
        engine.shutdown();
        Ok(())
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_job_with_filters() -> Result<()> {
        let mut engine = Engine::new().await;
        engine.ps();

        let job_str = r#"
            name = "test_filters"
            producer = {
                "Fetch" = [
                    "lux-me",
                    {
                        "from" = 1746898373376
                        "duration" = 3600
                        "delay" = 10
                    }
                ]
            }
            filters = [
                "Copy",
                { "Tee" = "copy.csv" }
            ]
            output = {
                "Save" = "output.csv"
            }
        "#;

        let job = engine.parse(job_str).await?;

        assert_eq!(job.middle.len(), 2);
        assert!(matches!(job.middle[0], Middle::Copy(_)));
        assert!(matches!(job.middle[1], Middle::Tee(_)));
        engine.shutdown();
        Ok(())
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_invalid_hcl() -> Result<()> {
        let mut engine = Engine::new().await;
        engine.ps();

        let job_str = r#"
            invalid hcl syntax
        "#;

        let result = engine.parse(job_str).await;
        assert!(result.is_err());
        engine.shutdown();
        Ok(())
    }
}
