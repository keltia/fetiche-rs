//! Compiler for the Fetiche job language
//!
//! Description of the job & task language -- leveraging HCL support.  The advantage is that we
//! could switch to JSON or anything else.
//!
//! See [this example](../../examples/parse.rs) for a simple usage example.
//!

use std::collections::VecDeque;

use eyre::Result;
use ractor::call;
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};
use tracing::{debug, trace};

use fetiche_common::Container;
use fetiche_formats::Format;

use crate::actors::{SchedulerMsg, SourcesMsg};
use crate::{
    Consumer, Copy, Engine, Fetch, Filter, Job, JobState, Middle, ParserError, Producer, Read,
    Save, Site, Store, Stream, Tee,
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
#[strum(serialize_all = "lowercase")]
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
    pub middle: Option<Vec<MiddleText>>,
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
    /// ```rust,no_run
    /// # #[tokio::main]
    /// # async fn main() -> eyre::Result<()> {
    /// # use fetiche_engine::Engine;
    /// # let mut engine = Engine::single().await?;
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
    #[tracing::instrument(skip(self))]
    pub async fn parse(&mut self, job_str: &str) -> Result<Job> {
        let jt: JobText = hcl::from_str(job_str)?;
        trace!("jobtext = {:?}", &jt);

        // Assign a new ID
        //
        let id = call!(self.scheduler, SchedulerMsg::Allocate)?;

        let mut job = Self::build_job(jt, id, async |site| {
            call!(self.sources, |port| SourcesMsg::Get(site, port))?
        })
        .await?;
        if let Producer::Read(reader) = &mut job.producer {
            reader.max_bytes(self.read_max_bytes);
        }
        Ok(job)
    }

    async fn build_job(
        jt: JobText,
        id: usize,
        mut lookup_site: impl AsyncFnMut(String) -> Result<Site>,
    ) -> Result<Job> {
        let producer = match jt.producer {
            ProducerText::Fetch(p, args) => {
                let site = lookup_site(p.clone()).await?;
                if site.is_fetchable() {
                    let mut f = Fetch::new(&jt.name);
                    f.site(site);
                    f.with(args);
                    Producer::Fetch(f.clone())
                } else {
                    return Err(ParserError::NotFetchable(p).into());
                }
            }
            ProducerText::Stream(p, args) => {
                let site = lookup_site(p.clone()).await?;
                if site.is_streamable() {
                    let mut s = Stream::new(&jt.name);
                    s.site(site);
                    s.with(args);
                    Producer::Stream(s.clone())
                } else {
                    return Err(ParserError::NotStreamable(p).into());
                }
            }
            ProducerText::Read(p) => {
                let mut r = Read::new(&p);
                r.path(&p);
                Producer::Read(r)
            }
        };

        debug!("Filters: {:?}", &jt.middle);
        // Lets have a look at filters now.
        //
        let list = if let Some(filters) = &jt.middle {
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

        // And finally at consumers.
        //
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
    use crate::Capability;

    async fn parse_job(input: &str) -> Result<Job> {
        Engine::build_job(hcl::from_str(input)?, 1, async |name: String| {
            let feature = match name.as_str() {
                "lux-me" => Capability::Fetch,
                "opensky" => Capability::Stream,
                _ => return Err(eyre::eyre!("Unknown source {name}")),
            };
            Ok(Site {
                name,
                feature,
                ..Site::default()
            })
        })
        .await
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_fetch_job() -> Result<()> {
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

        let job = parse_job(job_str).await?;

        assert_eq!(job.name, "test_fetch");
        assert_eq!(job.id, 1);
        assert_eq!(job.state(), JobState::Ready);
        assert!(job.middle.is_empty());
        let Producer::Fetch(fetch) = job.producer else {
            panic!("expected a Fetch producer");
        };
        assert_eq!(fetch.site.unwrap().name, "lux-me");
        assert_eq!(
            Filter::from(fetch.args.as_str()),
            Filter::stream(1746898373376, 3600, 10)
        );
        assert!(matches!(job.consumer, Consumer::Save(_)));

        Ok(())
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_stream_job() -> Result<()> {
        let job_str = r#"
            name = "test_stream"
            producer = {
                "Stream" = [
                    "opensky",
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

        let job = parse_job(job_str).await?;

        assert_eq!(job.name, "test_stream");
        let Producer::Stream(stream) = job.producer else {
            panic!("expected a Stream producer");
        };
        assert_eq!(stream.site.unwrap().name, "opensky");
        assert_eq!(
            Filter::from(stream.args.as_str()),
            Filter::stream(1746898373376, 3600, 10)
        );
        assert!(matches!(job.consumer, Consumer::Save(_)));
        Ok(())
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_read_job() -> Result<()> {
        let job_str = r#"
            name = "test_read"
            producer = {
                "Read" = "input.csv"
            }
            output = {
                "Save" = "output.csv"
            }
        "#;

        let job = parse_job(job_str).await?;

        assert_eq!(job.name, "test_read");
        let Producer::Read(reader) = job.producer else {
            panic!("expected a Read producer");
        };
        assert_eq!(reader.path.as_deref(), Some(std::path::Path::new("input.csv")));
        assert!(matches!(job.consumer, Consumer::Save(_)));
        Ok(())
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_job_with_filters() -> Result<()> {
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
            middle = [
                "Copy",
                { "Tee" = "copy.csv" }
            ]
            output = {
                "Save" = "output.csv"
            }
        "#;

        let job = parse_job(job_str).await?;

        assert_eq!(job.middle.len(), 2);
        assert!(matches!(job.middle[0], Middle::Copy(_)));
        let Middle::Tee(tee) = &job.middle[1] else {
            panic!("expected a Tee filter");
        };
        assert_eq!(tee.fname, "copy.csv");
        Ok(())
    }

    /// A known source with the wrong capability yields the operation-specific parser error.
    #[rstest::rstest]
    #[case("Fetch", "opensky")]
    #[case("Stream", "lux-me")]
    #[tokio::test]
    async fn test_parse_rejects_incompatible_source(#[case] operation: &str, #[case] site: &str) {
        let hcl = format!(
            r#"
            name = "incompatible_source"
            producer = {{ {operation} = ["{site}", 60] }}
            output = {{ Save = "output.csv" }}
            "#
        );

        let err = parse_job(&hcl).await.unwrap_err();
        match (operation, err.downcast_ref::<ParserError>()) {
            ("Fetch", Some(ParserError::NotFetchable(name)))
            | ("Stream", Some(ParserError::NotStreamable(name))) => assert_eq!(name, site),
            _ => panic!("unexpected error for {operation} from {site}: {err:?}"),
        }
    }

    /// Source lookup failures survive job construction with the source name intact.
    #[tokio::test]
    async fn test_parse_propagates_source_lookup_failure() {
        let err = parse_job(
            r#"
            name = "unknown_source"
            producer = { Fetch = ["missing", 60] }
            output = { Save = "output.csv" }
            "#,
        )
        .await
        .unwrap_err();
        assert_eq!(err.to_string(), "Unknown source missing");
    }

    /// Empty middle stages and the stdout destination produce a usable Save consumer.
    #[tokio::test]
    async fn test_parse_stdout_with_empty_middle() -> Result<()> {
        let job = parse_job(
            r#"
            name = "stdout"
            producer = { Read = "input.csv" }
            middle = []
            output = { Save = "-" }
            "#,
        )
        .await?;
        assert!(job.middle.is_empty());
        let Consumer::Save(save) = job.consumer else {
            panic!("expected a Save consumer");
        };
        assert!(save.path.is_none());
        Ok(())
    }

    /// A Store destination creates its job-specific directory during parsing.
    #[tokio::test]
    async fn test_parse_store_job() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().to_string_lossy().replace('\\', "/");
        let hcl = format!(
            r#"
            name = "store"
            producer = {{ Read = "input.csv" }}
            output = {{ Store = ["{path}", "Hourly"] }}
            "#
        );

        let job = parse_job(&hcl).await?;
        assert!(matches!(job.consumer, Consumer::Store(_)));
        assert!(dir.path().join(job.id.to_string()).is_dir());
        Ok(())
    }

    /// Store construction errors reach the parser caller.
    #[tokio::test]
    async fn test_parse_rejects_empty_store_path() {
        let err = parse_job(
            r#"
            name = "empty_store"
            producer = { Read = "input.csv" }
            output = { Store = ["", "Hourly"] }
            "#,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err.downcast_ref::<crate::EngineStatus>(),
            Some(crate::EngineStatus::NoPathDefined)
        ));
    }

    /// Valid HCL with missing or malformed job fields fails during deserialization.
    #[rstest::rstest]
    #[case(r#"producer = { Read = "input.csv" }"#)]
    #[case(r#"producer = { Unknown = "input.csv" }
        output = { Save = "output.csv" }"#)]
    #[case(r#"producer = { Read = "input.csv" }
        output = { Store = ["data", "Weekly"] }"#)]
    #[tokio::test]
    async fn test_parse_rejects_invalid_job_fields(#[case] fields: &str) {
        let hcl = format!("name = \"invalid\"\n{fields}");
        let err = parse_job(&hcl).await.unwrap_err();
        assert!(err.downcast_ref::<hcl::Error>().is_some(), "{err:?}");
    }

    #[test_pretty_log::test]
    #[tokio::test]
    async fn test_parse_invalid_hcl() -> Result<()> {
        let job_str = r#"
            invalid hcl syntax
        "#;

        let result = parse_job(job_str).await;
        assert!(result.is_err());
        Ok(())
    }
}
