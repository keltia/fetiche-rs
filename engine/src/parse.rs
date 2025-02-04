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

use eyre::Result;
use fetiche_formats::Format;
use ractor::call;
use ractor::rpc::call;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use strum::EnumString;
use tracing::trace;

use crate::actors::{QueueMsg, SourcesMsg};
use crate::{Engine, Fetch, Job, JobState, Read, Stream, Task};

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
pub enum JobType {
    /// One-shot fetch a block of data.
    Fetch,
    /// Read a local file.
    Read,
    /// Long-running job, streaming.
    Stream,
}

/// Represents the various types of filters that can be applied to a job.
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
pub enum Filter {
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
pub enum Consumer {
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

/// Represents the main entry point for defining and executing tasks in the Fetiche engine.
///
/// This module provides structures and methods to define jobs, filters, and job types
/// using the Fetiche job language. The job language is designed to allow declarative 
/// specifications of how data is fetched, processed, and output. It supports using HCL as 
/// the primary configuration language, while allowing flexibility to switch to JSON or 
/// other formats in the future.
///
/// # Features
///
/// - **Job Types:** Enumerates the type of tasks that can be executed (e.g., fetching data, reading files or streaming data in real-time).
/// - **Filters:** Allows defining additional transformation or processing applied on jobs.
/// - **HCL Parsing:** Conversion from HCL job definitions into structured `JobStruct`.
///
/// # Examples
///
/// ## Job Definition in HCL
///
/// A job definition in the Fetiche language may look like:
///
/// ```hcl
/// name = "Opensky"
/// type = "fetch"
/// source = opensky
/// output = "foo.csv"
/// ```
///
/// ## Defining and Parsing a Job in Rust
///
/// Here's a Rust example of defining a job with HCL and parsing it:
///
/// ```rust
/// # use nom::Parser;
/// use fetiche_engine::{Engine, JobText, JobType};
///
/// let hcl_input = r#"
/// name = "Fetch Job"
/// type = "fetch"
/// source = "opensky"
/// output = { "Save" = "data.csv") }
/// "#;
///
/// let mut engine = Engine::new();
/// let job: JobText = engine.parse(hcl_input).expect("Failed to parse job")?;
///
/// assert_eq!(job.name, "Fetch Job");
/// assert_eq!(job.jtype, JobType::Fetch);
/// ```
///
/// # Details
///
/// The job consists of multiple components:
/// - **Job Types (`JobType`):** Defines the type of tasks that can be performed.
/// - **Filters (`Filter`):** Specifies optional processing or transformation to the output.
/// - **Job Structure (`JobStruct`):** Captures all details of the job into a structured format parsed from HCL.
///
/// This module makes heavy use of the `serde` crate for serialization/deserialization and the `hcl` crate for configuration parsing.
///
#[derive(Debug, Deserialize, Serialize)]
pub struct JobText {
    #[serde(rename = "type")]
    pub jtype: JobType,
    /// Job name.
    pub name: String,
    /// Source (aka Site name).
    pub source: String,
    /// Optional list of filters like `Tee` or `Save`.
    pub filters: Option<Vec<Filter>>,
    /// Output file name.
    pub output: Consumer,
}

impl Engine {
    /// Our job structure is
    /// - one producer
    /// - zero or more filters
    /// - one consumer
    ///
    pub fn parse(&mut self, job_str: &str) -> Result<Job> {
        let jt: JobText = hcl::from_str(job_str)?;
        trace!("{:?}", &jt);

        // Retrieve the site's data from the Sources actor.
        //
        let site = call!(self.sources, |port| SourcesMsg::Get(jt.name, port))?;
        let producer = match jt.jtype {
            JobType::Fetch => {
                Task::from(Fetch::new(&jt.name).site(site))
            }
            JobType::Stream => {
                Task::from(Stream::new(&jt.name).site(site))
            }
            JobType::Read => {
                Task::from(Read::new(&jt.name).path(&site.name))
            }
        };
        let mut list = VecDeque::from([producer]);
        if let Some(filters) = &jt.filters {
            filters.iter().for_each(|t| list.push_back(Task::from(t)))
        }
        list.push_back(Task::from(jt.output));

        let id = call!(self.queue, |port| QueueMsg::Allocate(port))?;

        // Job is now Ready as it is complete with task list.
        //
        let job = Job {
            id,
            name: jt.name.clone(),
            state: JobState::Ready,
            list,
        };
        Ok(job)
    }
}
