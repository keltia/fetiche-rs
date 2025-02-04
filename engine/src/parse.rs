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
use ractor::rpc::call;
use serde::{Deserialize, Serialize};
use strum::EnumString;
use tracing::trace;

use crate::actors::{QueueMsg, SourcesMsg};
use crate::{Consumer, Convert, Copy, Engine, Fetch, Job, JobState, Middle, Producer, Read, Save, Store, Stream, Tee};

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
#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
pub enum Freq {
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
                let site = call!(self.sources, |port| SourcesMsg::Get(jt.name, port))?;
                let f = Fetch::new(&p).site(site);
                Producer::Fetch(f.clone())
            }
            ProducerText::Stream(p) => {
                let site = call!(self.sources, |port| SourcesMsg::Get(jt.name, port))?;
                let s = Stream::new(&p).site(site);
                Producer::Stream(s.clone())
            }
            ProducerText::Read(p) => {
                let r = Read::new(&p);
                Producer::Read(r)
            }
        };
        let mut list = if let Some(filters) = &jt.filters {
            filters.iter().map(|t| t.clone()).collect()
        } else {
            VecDeque::new()
        };
        let consumer = match jt.output {
            ConsumerText::Archive(c) => {
                Consumer::Invalid
            }
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
        };
        Ok(job)
    }
}
