//! Compiler for the Fetiche job language
//!
//! Description of the job & task language -- leveraging HCL support.  The advantage is that we
//! could switch to JSON or anything else.
//!
//! See [`Engine::parse`] for a usage example.
//!

use std::collections::VecDeque;
use std::path::Path;

use eyre::Result;
use ractor::call;
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};
use tracing::debug;

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
/// deserialized from HCL variant names such as `Fetch` and `Read`.
///
/// # Variants
///
/// - `Fetch`: Fetches data from an external source.
/// - `Read`: Reads data from an existing file or resource.
/// - `Stream`: Streams data directly from an external source in real-time.
///
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
enum ProducerText {
    /// One-shot fetch a block of data.
    Fetch(String, Filter),
    /// Read a local file using a literal path.
    Read(hcl::Expression),
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
/// - `Copy`: Forwards its input.
/// - `Convert`: Reserved for a future implementation and rejected by the job parser.
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
    /// `Read` requires a nonempty literal string. The engine does not evaluate
    /// HCL expressions or interpolate templates in file paths. `Convert` and
    /// `Archive` are rejected before a job is ready to submit.
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
    #[tracing::instrument(skip(self, job_str))]
    pub async fn parse(&mut self, job_str: &str) -> Result<Job> {
        let jt: JobText = hcl::from_str(job_str)?;

        // Assign a new ID
        //
        let id = call!(self.scheduler, SchedulerMsg::Allocate)?;

        let mut job = Self::build_job(jt, id, &self.workdir, async |site| {
            call!(self.sources, |port| SourcesMsg::Get(site, port))?
        })
        .await?;
        if let Producer::Read(reader) = &mut job.producer {
            reader.max_bytes(self.read_max_bytes);
        }
        Ok(job)
    }

    /// Builds a ready job after validating its source, paths, and stage kinds.
    /// A Store directory is created only after the earlier stages validate.
    async fn build_job(
        jt: JobText,
        id: usize,
        workspace: &Path,
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
                let hcl::Expression::String(p) = p else {
                    return Err(ParserError::InvalidReadPath.into());
                };
                if p.is_empty() {
                    return Err(ParserError::InvalidReadPath.into());
                }
                let mut r = Read::new(&p);
                r.path(&p);
                Producer::Read(r)
            }
        };

        debug!(job_id = id, "Building job stages");
        // Lets have a look at filters now.
        //
        let list = jt
            .middle
            .unwrap_or_default()
            .into_iter()
            .map(|stage| match stage {
                MiddleText::Convert(_) => Err(ParserError::UnsupportedStage("Convert")),
                MiddleText::Copy => Ok(Middle::Copy(Copy::new())),
                MiddleText::Tee(fname) => Ok(Middle::Tee(Tee::into(&fname))),
            })
            .collect::<Result<VecDeque<_>, _>>()?;

        // And finally at consumers.
        //
        let consumer = match jt.output {
            ConsumerText::Archive(_) => return Err(ParserError::UnsupportedStage("Archive").into()),
            ConsumerText::Save(c) => {
                let mut f = Save::new(&c, Format::None, Container::Raw);
                f.path(&c);
                Consumer::Save(f)
            }
            ConsumerText::Store(c, f) => {
                let path = if c.is_empty() {
                    c
                } else {
                    let path = crate::workspace::resolve_stage_path(workspace, Path::new(&c))?;
                    path.to_str()
                        .ok_or_else(|| {
                            eyre::eyre!("storage path is not UTF-8: {}", path.display())
                        })?
                        .to_owned()
                };
                let s = Store::new(&path, id, f).await?;
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
        debug!(job_id = job.id, "Parsed job");
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Capability;

    /// Captures emitted spans and events while a job with fake credentials is built.
    #[cfg(feature = "asd")]
    #[tokio::test]
    async fn test_job_and_token_traces_omit_credentials() -> Result<()> {
        use crate::token::AsdToken;
        use crate::{Auth, TokenStorage, TokenType, Work};
        use std::io::{self, Write};
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::format::FmtSpan;

        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<u8>>>);

        impl Write for Capture {
            /// Appends a formatted trace record to the shared test buffer.
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().write(bytes)
            }

            /// The in-memory buffer is immediately visible to the test.
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let capture = Capture::default();
        let writer = capture.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::TRACE)
            .with_span_events(FmtSpan::NEW)
            .finish();
        let _subscriber_guard = tracing::subscriber::set_default(subscriber);

        let input = hcl::from_str(
            r#"
            name = "trace-job"
            producer = { Fetch = ["trace-source", { name = "key", value = "fake-filter-secret" }] }
            output = { Save = "output.csv" }
            "#,
        )?;
        let job = Engine::build_job(input, 901, Path::new("."), async |name| {
            Ok(Site {
                name,
                feature: Capability::Fetch,
                base_url: "https://example.test/?key=fake-url-secret".into(),
                auth: Some(Auth::Token {
                    login: "fake-login-secret".into(),
                    password: "fake-password-secret".into(),
                    token: "fake-source-token".into(),
                }),
                ..Site::default()
            })
        })
        .await?;
        let Producer::Fetch(fetch) = &job.producer else {
            panic!("expected a Fetch producer");
        };
        let site = fetch.site.as_ref().unwrap();
        let formatted_source = format!("{site} {site:?}");
        assert!(formatted_source.contains("trace-source"));
        let (completion, _outcome) = tokio::sync::oneshot::channel();
        let _work = Work::new(job, completion);

        let mut token = AsdToken::new();
        token.token = "fake-stored-token".into();
        let json = serde_json::to_string(&token)?;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("fake-login-secret");
        AsdToken::store(&path, &json)?;
        let token = AsdToken::retrieve(&path)?;
        assert_eq!(token.token, "fake-stored-token");
        AsdToken::purge(&path)?;
        let mut tokens = TokenStorage::register(&dir.path().to_path_buf()).await?;
        tokens
            .store("asd_fake-login-secret", TokenType::AsdToken(token))
            .await?;

        let captured = capture.0.lock().unwrap();
        let logs = std::str::from_utf8(&captured)?;
        assert!(
            logs.contains("job_id=901") && logs.contains("trace-source"),
            "{logs}"
        );
        for secret in [
            "fake-filter-secret",
            "fake-url-secret",
            "fake-login-secret",
            "fake-password-secret",
            "fake-source-token",
            "fake-stored-token",
        ] {
            assert!(!logs.contains(secret), "trace exposed {secret}");
            assert!(
                !formatted_source.contains(secret),
                "source format exposed {secret}"
            );
        }
        Ok(())
    }

    async fn parse_job(input: &str) -> Result<Job> {
        Engine::build_job(
            hcl::from_str(input)?,
            1,
            Path::new("."),
            async |name: String| {
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
            },
        )
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
        assert_eq!(
            reader.path.as_deref(),
            Some(std::path::Path::new("input.csv"))
        );
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
    #[case(
        r#"producer = { Unknown = "input.csv" }
        output = { Save = "output.csv" }"#
    )]
    #[case(
        r#"producer = { Read = "input.csv" }
        output = { Store = ["data", "Weekly"] }"#
    )]
    #[tokio::test]
    async fn test_parse_rejects_invalid_job_fields(#[case] fields: &str) {
        let hcl = format!("name = \"invalid\"\n{fields}");
        let err = parse_job(&hcl).await.unwrap_err();
        assert!(err.downcast_ref::<hcl::Error>().is_some(), "{err:?}");
    }

    /// Read accepts literal paths and rejects HCL expressions, templates, and empty paths.
    #[rstest::rstest]
    #[case("42")]
    #[case("var.input")]
    #[case(r#""prefix-${var.input}.csv""#)]
    #[case(r#""""#)]
    #[tokio::test]
    async fn test_parse_rejects_invalid_read_path(#[case] path: &str) {
        let hcl = format!(
            r#"
            name = "read"
            producer = {{ Read = {path} }}
            output = {{ Save = "-" }}
            "#
        );
        let err = parse_job(&hcl).await.unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<ParserError>(),
                Some(ParserError::InvalidReadPath)
            ),
            "{err:?}"
        );
    }

    /// Quoting a numeric filename preserves it as a path.
    #[tokio::test]
    async fn test_parse_accepts_numeric_literal_filename() -> Result<()> {
        let job = parse_job(
            r#"
            name = "read"
            producer = { Read = "42" }
            output = { Save = "-" }
        "#,
        )
        .await?;
        let Producer::Read(read) = job.producer else {
            panic!("expected Read")
        };
        assert_eq!(read.path.as_deref(), Some(Path::new("42")));
        Ok(())
    }

    /// Unsupported Convert fails before Store creates a directory.
    #[tokio::test]
    async fn test_parse_rejects_convert_before_store_setup() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let text = hcl::from_str(
            r#"
            name = "unsupported conversion"
            producer = { Read = "input.csv" }
            middle = [{ Convert = "DronePoint" }]
            output = { Store = ["output", "Hourly"] }
        "#,
        )?;
        let err = Engine::build_job(text, 1, dir.path(), |_| async {
            panic!("Read should not resolve a source")
        })
        .await
        .unwrap_err();
        assert!(matches!(
            err.downcast_ref::<ParserError>(),
            Some(ParserError::UnsupportedStage("Convert"))
        ));
        assert!(!dir.path().join("output").exists());
        Ok(())
    }

    /// Archive returns a parser error rather than creating an invalid consumer.
    #[tokio::test]
    async fn test_parse_rejects_archive() {
        let err = parse_job(
            r#"
            name = "unsupported archive"
            producer = { Read = "input.csv" }
            output = { Archive = "old-data" }
        "#,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err.downcast_ref::<ParserError>(),
            Some(ParserError::UnsupportedStage("Archive"))
        ));
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
