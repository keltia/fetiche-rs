//! Local Engine client library for Fetiche.
//!
//! # Overview
//!
//! The client library provides the core data structures and types for defining and configuring
//! data processing jobs in the Fetiche engine. It acts as an interface layer between the user's
//! job definitions and the engine's execution environment.
//!
//! # Components
//!
//! - [`JobText`]: The main job configuration structure that defines the complete processing pipeline
//! - [`ProducerText`]: Defines how data is sourced (fetch, read, or stream)
//! - [`MiddleText`]: Specifies intermediate processing steps (conversion, copying, teeing)
//! - [`ConsumerText`]: Determines how the processed data is output (archive, save, or store)
//! - [`Freq`]: Defines scheduling frequencies for certain operations
//!
//! # Example
//!
//! ```rust
//! use fetiche_client::{ConsumerText, JobText, ProducerText};
//!
//! let job = JobText::builder()
//!     .name("example_job")
//!     .producer(ProducerText::Read("input.txt".to_string()))
//!     .output(ConsumerText::Save("output.txt".to_string()))
//!     .build();
//! ```
//!

use bon::Builder;
use eyre::Result;
use serde::{Deserialize, Serialize};
use strum::EnumString;

use fetiche_common::Filter;
use fetiche_engine::ParserError;
use fetiche_formats::Format;

pub use fetiche_engine::Freq;

/// Defines the input, optional processing steps, and output of a job.
///
/// Use [`Self::builder`] to construct a definition and
/// [`crate::EngineSingle::parse_job`] to turn it into an engine job.
///
/// # Examples
///
/// ```
/// use fetiche_client::{ConsumerText, JobText, ProducerText};
///
/// let job = JobText::builder()
///     .name("copy_file")
///     .producer(ProducerText::Read("input.txt".into()))
///     .output(ConsumerText::Save("output.txt".into()))
///     .build();
/// assert_eq!(job.name, "copy_file");
/// ```
///
#[derive(Builder, Debug, Deserialize, Serialize)]
#[builder(
    builder_type(doc {
        /// Fluent builder returned by [`JobText::builder`].
        ///
    }),
    start_fn(doc {
        /// Starts a builder for a job definition.
        ///
    }),
    finish_fn(doc {
        /// Returns the job definition with defaults for omitted fields.
        ///
        /// Set a producer and an output before passing the definition to
        /// [`crate::EngineSingle::parse_job`]. Both default to invalid placeholders.
        ///
    })
)]
pub struct JobText {
    /// Job name.
    ///
    /// Defaults to an empty string.
    ///
    #[builder(into, default = String::new())]
    pub name: String,
    /// Data generator
    ///
    /// Defaults to [`ProducerText::Invalid`].
    ///
    #[builder(default = ProducerText::default())]
    pub producer: ProducerText,
    /// Optional list of filters like `Tee` or `Save`.
    ///
    /// Steps run in list order. Use [`MiddleText::Tee`] to save a copy
    /// and [`ConsumerText::Save`] to save the final output.
    ///
    pub middle: Option<Vec<MiddleText>>,
    /// Output file name.
    ///
    /// Defaults to [`ConsumerText::Invalid`]. The output variant determines
    /// which destination settings apply.
    ///
    #[builder(default = ConsumerText::default())]
    pub output: ConsumerText,
}

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
/// For serialization and deserialization through Serde, use the case-sensitive
/// variant names, such as `Fetch` and `Read`.
///
#[derive(Clone, Debug, Default, Deserialize, EnumString, PartialEq, Serialize)]
pub enum ProducerText {
    /// One-shot fetch a block of data.
    ///
    /// Takes the configured source name and its filter.
    ///
    Fetch(String, Filter),
    /// Read a local file.
    ///
    Read(String),
    /// Long-running job, streaming.
    ///
    /// Takes the configured source name and its filter.
    ///
    Stream(String, Filter),
    /// Default placeholder for a job definition with no selected input.
    ///
    #[default]
    Invalid,
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
/// The current enum exposes [`Self::Convert`], [`Self::Copy`], and [`Self::Tee`]
/// as processing options. Use [`ConsumerText::Save`] for the final output.
///
#[derive(Clone, Debug, Default, Deserialize, EnumString, PartialEq, Serialize)]
pub enum MiddleText {
    //// Conversion between formats.
    /// Requests conversion to the given format.
    ///
    /// Parsing a job with this step currently panics.
    ///
    Convert(Format),
    /// Block by block copy.
    ///
    Copy,
    /// Duplicate the data in a given file.
    ///
    /// Forwards the data downstream as it writes the copy.
    ///
    Tee(String),
    /// Default placeholder for an unspecified processing step.
    ///
    #[default]
    Invalid,
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
/// [`Self::Store`] also takes a [`Freq`] that controls file rotation.
///
#[derive(Clone, Debug, Default, Deserialize, EnumString, PartialEq, Serialize)]
pub enum ConsumerText {
    /// Archive multiple files in a single one.
    ///
    /// The engine parser currently maps this option to an invalid consumer.
    ///
    Archive(String),
    /// Save in a file.
    ///
    Save(String),
    /// Store files by frequency in the specified directory.
    ///
    Store(String, Freq),
    /// Default placeholder for a job definition with no selected output.
    ///
    #[default]
    Invalid,
}

/// Source operation selected by [`JobBuilder`].
///
#[derive(Clone, Debug, Default)]
pub enum JobType {
    /// Fetches a batch of data from the named source.
    ///
    Fetch(String),
    /// Streams data from the named source.
    ///
    Stream(String),
    /// Default state before a source operation has been selected.
    ///
    #[default]
    Invalid,
}

/// A builder for constructing data processing jobs with a fluent interface.
///
/// `JobBuilder` provides methods to configure various aspects of a job:
/// * Job identification through a name
/// * Data source configuration (fetch or stream)
/// * Data filtering capabilities
/// * Middleware operations like tee
/// * Output handling (save or store)
///
/// All configuration is done through method chaining, with the final job
/// being created by calling `build()`.
///
/// Select a source with [`Self::fetch`] or [`Self::stream`], set a filter with
/// [`Self::filter`], and select an output with [`Self::save`] or [`Self::store`]
/// before calling [`Self::build`].
///
/// # Examples
///
/// ```
/// use fetiche_client::{Filter, JobBuilder};
///
/// let job = JobBuilder::new("fetch_data")
///     .fetch("configured_source")
///     .filter(Filter::default())
///     .save("output.csv")
///     .build()?;
/// # Ok::<(), eyre::Report>(())
/// ```
///
#[derive(Debug, Clone, Default)]
pub struct JobBuilder {
    /// Job description
    name: String,
    /// This is the job type with the embedded site name
    producer: JobType,
    /// Filter to apply to the data
    filter: Option<Filter>,
    /// Middleware operations like tee
    middle: Vec<MiddleText>,
    /// Output handling (save or store)
    output: Option<ConsumerText>,
}

impl JobBuilder {
    /// Creates a new JobBuilder with the specified name.
    ///
    /// # Arguments
    /// * `name` - The name of the job to be created
    ///
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Configures the job to fetch data from a specified site.
    ///
    /// # Arguments
    /// * `site` - The URL or identifier of the site to fetch from
    ///
    /// The engine looks up the source by its configured name. This replaces the previous source selection.
    ///
    #[tracing::instrument(skip(self))]
    pub fn fetch(&mut self, site: &str) -> &mut Self {
        self.producer = JobType::Fetch(site.to_string());
        self
    }

    /// Configures the job to stream data from a specified site.
    ///
    /// # Arguments
    /// * `site` - The URL or identifier of the site to stream from
    ///
    /// The engine looks up the source by its configured name. This replaces the previous source selection.
    ///
    #[tracing::instrument(skip(self))]
    pub fn stream(&mut self, site: &str) -> &mut Self {
        self.producer = JobType::Stream(site.to_string());
        self
    }

    /// Adds a filter to the job for processing data.
    ///
    /// # Arguments
    /// * `f` - The filter to apply to the data
    ///
    /// This replaces any previous filter.
    ///
    #[tracing::instrument(skip(self))]
    pub fn filter(&mut self, f: Filter) -> &mut Self {
        self.filter = Some(f);
        self
    }

    /// Adds a tee operation to write data to a file while passing it through.
    ///
    /// # Arguments
    /// * `fname` - The name of the file to write to
    ///
    /// Passing `None` skips adding a processing step.
    ///
    #[tracing::instrument(skip(self))]
    pub fn tee(&mut self, fname: Option<String>) -> &mut Self {
        if let Some(fname) = fname {
            self.middle.push(MiddleText::Tee(fname.clone()));
        }
        self
    }

    /// Configures the job to save its output to a file.
    ///
    /// # Arguments
    /// * `fname` - The name of the file to save to
    ///
    /// This replaces any previous output.
    ///
    #[tracing::instrument(skip(self))]
    pub fn save(&mut self, fname: &str) -> &mut Self {
        self.output = Some(ConsumerText::Save(fname.to_string()));
        self
    }

    /// Configures the job to store its output with specified frequency.
    ///
    /// # Arguments
    /// * `path` - The path where data should be stored
    /// * `freq` - The frequency at which data should be stored
    ///
    /// This replaces any previous output.
    ///
    #[tracing::instrument(skip(self))]
    pub fn store(&mut self, path: &str, freq: Freq) -> &mut Self {
        self.output = Some(ConsumerText::Store(path.to_string(), freq));
        self
    }

    /// Builds the final job configuration.
    ///
    /// The builder retains its settings for subsequent calls. The engine resolves
    /// the source and sets up the pipeline when the definition is parsed.
    ///
    /// # Errors
    ///
    /// Returns an error if neither [`Self::fetch`] nor [`Self::stream`] has been selected.
    ///
    /// # Panics
    ///
    /// Panics if a source is selected but the filter or output is missing.
    ///
    #[tracing::instrument(skip(self))]
    pub fn build(&mut self) -> Result<JobText> {
        let producer = match &self.producer {
            JobType::Fetch(site) => ProducerText::Fetch(site.clone(), self.filter.clone().unwrap()),
            JobType::Stream(site) => {
                ProducerText::Stream(site.clone(), self.filter.clone().unwrap())
            }
            _ => return Err(ParserError::InvalidJobType.into()),
        };

        Ok(JobText::builder()
            .name(self.name.clone())
            .producer(producer)
            .maybe_middle(Some(self.middle.clone()))
            .output(self.output.clone().unwrap())
            .build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fetiche_engine::Freq;
    use proptest::prelude::*;

    #[test]
    fn test_new_job_builder() {
        let builder = JobBuilder::new("test_job");
        assert_eq!(builder.name, "test_job");
        assert!(matches!(builder.producer, JobType::Invalid));
    }

    #[test]
    fn test_fetch_job() -> Result<()> {
        let mut builder = JobBuilder::new("test_fetch");
        let filter = Filter::default();
        builder.fetch("somesite").filter(filter);
        let job = builder.save("file").build()?;
        assert_eq!(job.name, "test_fetch");
        assert!(matches!(job.producer, ProducerText::Fetch(_, _)));
        Ok(())
    }

    #[test]
    fn test_stream_job() -> Result<()> {
        let mut builder = JobBuilder::new("test_stream");
        let filter = Filter::default();
        builder.stream("anothersite").filter(filter);
        let job = builder.save("file").build()?;
        assert_eq!(job.name, "test_stream");
        assert!(matches!(job.producer, ProducerText::Stream(_, _)));
        Ok(())
    }

    #[test]
    fn test_tee_middleware() -> Result<()> {
        let mut builder = JobBuilder::new("test_tee");
        let filter = Filter::default();
        builder
            .fetch("https://example.com")
            .filter(filter)
            .tee(Some("output.txt".to_string()))
            .save("final.txt");
        let job = builder.build()?;
        assert_eq!(job.middle.clone().unwrap().len(), 1);
        assert!(matches!(job.middle.unwrap()[0], MiddleText::Tee(_)));
        Ok(())
    }

    #[test]
    fn test_save_output() -> Result<()> {
        let mut builder = JobBuilder::new("test_save");
        let filter = Filter::default();
        builder
            .fetch("https://example.com")
            .filter(filter)
            .save("output.txt");
        let job = builder.build()?;
        assert!(matches!(job.output, ConsumerText::Save(_)));
        Ok(())
    }

    #[test]
    fn test_store_output() -> Result<()> {
        let mut builder = JobBuilder::new("test_store");
        let filter = Filter::default();
        builder
            .fetch("https://example.com")
            .filter(filter)
            .store("data", Freq::Daily);
        let job = builder.build()?;
        assert!(matches!(job.output, ConsumerText::Store(_, _)));
        Ok(())
    }

    #[test]
    fn test_job_text_builder() {
        let job = JobText::builder()
            .name("test_job")
            .producer(ProducerText::Read("input.txt".to_string()))
            .build();

        assert_eq!(job.name, "test_job");
        assert!(matches!(job.producer, ProducerText::Read(_)));
    }

    #[test]
    fn test_producer_text_variants() {
        let fetch = ProducerText::Fetch("url".to_string(), Filter::default());
        let read = ProducerText::Read("file.txt".to_string());
        let stream = ProducerText::Stream("stream".to_string(), Filter::default());

        assert!(matches!(fetch, ProducerText::Fetch(_, _)));
        assert!(matches!(read, ProducerText::Read(_)));
        assert!(matches!(stream, ProducerText::Stream(_, _)));
    }

    #[test]
    fn test_middle_text_variants() {
        let convert = MiddleText::Convert(Format::Cat21);
        let copy = MiddleText::Copy;
        let tee = MiddleText::Tee("output.txt".to_string());

        assert!(matches!(convert, MiddleText::Convert(_)));
        assert!(matches!(copy, MiddleText::Copy));
        assert!(matches!(tee, MiddleText::Tee(_)));
    }

    #[test]
    fn test_consumer_text_variants() {
        let archive = ConsumerText::Archive("archive.zip".to_string());
        let save = ConsumerText::Save("output.txt".to_string());
        let store = ConsumerText::Store("dir".to_string(), Freq::Daily);

        assert!(matches!(archive, ConsumerText::Archive(_)));
        assert!(matches!(save, ConsumerText::Save(_)));
        assert!(matches!(store, ConsumerText::Store(_, _)));
    }

    #[test]
    fn test_freq_variants() {
        assert!(matches!(Freq::default(), Freq::Daily));
        assert!(matches!(Freq::Hourly, Freq::Hourly));
    }

    proptest! {
        #[test]
        fn test_job_text_builder_prop(name: String) {
            let job = JobText::builder()
                .name(&name)
                .producer(ProducerText::Read("input.txt".to_string()))
                .build();

            prop_assert_eq!(job.name, name);
        }

        #[test]
        fn test_producer_text_serde_prop(s: String) {
            let producer = ProducerText::Read(s.clone());
            let serialized = serde_json::to_string(&producer).unwrap();
            let deserialized: ProducerText = serde_json::from_str(&serialized).unwrap();

            prop_assert_eq!(producer, deserialized);
        }

        #[test]
        fn test_middle_text_serde_prop(s: String) {
            let middle = MiddleText::Tee(s.clone());
            let serialized = serde_json::to_string(&middle).unwrap();
            let deserialized: MiddleText = serde_json::from_str(&serialized).unwrap();

            prop_assert_eq!(middle, deserialized);
        }

        #[test]
        fn test_consumer_text_serde_prop(s: String) {
            let consumer = ConsumerText::Save(s.clone());
            let serialized = serde_json::to_string(&consumer).unwrap();
            let deserialized: ConsumerText = serde_json::from_str(&serialized).unwrap();

            prop_assert_eq!(consumer, deserialized);
        }
    }
}
