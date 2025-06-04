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
//! use acutectl::{ConsumerText, JobTextBuilder, ProducerText};
//!
//! let job = JobTextBuilder::default()
//!     .name("example_job")
//!     .producer(ProducerText::Read("input.txt".to_string()))
//!     .output(ConsumerText::Save("output.txt".to_string()))
//!     .build()
//!     .unwrap();
//! ```
//!

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};

use fetiche_engine::Filter;
use fetiche_formats::Format;

#[derive(Builder, Debug, Deserialize, Serialize)]
pub struct JobText {
    /// Job name.
    #[builder(setter(into), default = "String::new()")]
    pub name: String,
    /// Data generator
    #[builder(default = "ProducerText::default()")]
    pub producer: ProducerText,
    /// Optional list of filters like `Tee` or `Save`.
    #[builder(default = "Some(Vec::new())")]
    pub middle: Option<Vec<MiddleText>>,
    /// Output file name.
    #[builder(default = "ConsumerText::default()")]
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
#[derive(Clone, Debug, Default, Deserialize, EnumString, PartialEq, Serialize)]
pub enum ProducerText {
    /// One-shot fetch a block of data.
    Fetch(String, Filter),
    /// Read a local file.
    Read(String),
    /// Long-running job, streaming.
    Stream(String, Filter),
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
#[derive(Clone, Debug, Default, Deserialize, EnumString, PartialEq, Serialize)]
pub enum MiddleText {
    //// Conversion between formats.
    Convert(Format),
    /// Block by block copy.
    Copy,
    /// Duplicate the data in a given file.
    Tee(String),
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
#[derive(Clone, Debug, Default, Deserialize, EnumString, PartialEq, Serialize)]
pub enum ConsumerText {
    /// Archive multiple files in a single one.
    Archive(String),
    /// Save in a file.
    Save(String),
    /// Store files by frequency in the specified directory.
    Store(String, Freq),
    #[default]
    Invalid,
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

#[cfg(test)]
mod tests {
    use super::*;
    use fetiche_engine::Filter;
    use proptest::prelude::*;

    #[test]
    fn test_job_text_builder() {
        let job = JobTextBuilder::default()
            .name("test_job")
            .producer(ProducerText::Read("input.txt".to_string()))
            .build()
            .unwrap();

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
            let job = JobTextBuilder::default()
                .name(&name)
                .producer(ProducerText::Read("input.txt".to_string()))
                .build();

            prop_assert!(job.is_ok());
            prop_assert_eq!(job.unwrap().name, name);
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
