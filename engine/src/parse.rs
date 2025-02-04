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

use eyre::Result;
use ractor::call;
use serde::{Deserialize, Serialize};
use strum::EnumString;

use crate::actors::SourcesMsg;
use crate::Engine;

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
#[derive(Clone, Debug, Deserialize, EnumString, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    Fetch,
    Read,
    Stream,
}

/// A representation of a job in the Fetiche language.
///
/// This struct defines the various components that describe a job to be executed
/// within the system. Jobs are defined using a language derived from HCL for flexibility.
///
/// # Example
///
/// ```hcl
/// name = "Opensky"
/// type = "fetch"
/// source = "opensky"
/// output = "foo.csv"
/// ```
///
/// # Fields
/// - `jtype` - The type of job to be performed. This can be one of:
///   - `fetch`: Fetch data from the specified source.
///   - `read`: Read data from a file or another medium.
///   - `stream`: Stream data from the source.
/// - `name` - The name of the job.
/// - `source` - The data source for the job.
/// - `tee` - An optional field specifying a data stream duplication target.
/// - `split` - An optional field specifying a path to split job output.
/// - `save` - An optional field specifying a path to save intermediate results.
/// - `output` - The path or name of the output file.
///
/// This struct is parsed from an HCL-formatted input.
///
#[derive(Debug, Deserialize, Serialize)]
pub struct JobStruct {
    #[serde(rename = "type")]
    pub jtype: JobType,
    pub name: String,
    pub source: String,
    pub tee: Option<String>,
    pub split: Option<String>,
    pub save: Option<String>,
    pub output: String,
}

impl Engine {
    /// Parses a job definition written in HCL and converts it into a `JobStruct`.
    ///
    /// # Arguments
    ///
    /// - `job_str`: A string slice containing the HCL job definition to be parsed.
    ///
    /// # Returns
    ///
    /// If successful, this function returns a `Result` containing a `JobStruct`
    /// that represents the parsed job definition. If an error occurs during
    /// parsing or data retrieval, an error encapsulated in `eyre::Result` is returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// use nom::Parser;
    /// use fetiche_engine::{Engine, JobStruct};
    ///
    /// let mut engine = Engine::new();
    /// let hcl_input = r#"
    /// name = "Fetch Test Job"
    /// type = "fetch"
    /// source = "test_source"
    /// output = "test_output.csv"
    /// "#;
    ///
    /// let job: JobStruct = engine.parse(hcl_input).expect("Failed to parse job")?;
    /// assert_eq!(job.name, "Fetch Test Job");
    /// ```
    ///
    /// # Errors
    ///
    /// This function may return an error if:
    /// - The input string is not valid HCL.
    /// - The fields in the HCL do not match the expected structure.
    /// - There is an issue in communication with the `sources` actor.
    ///
    pub fn parse(&mut self, job_str: &str) -> Result<JobStruct> {
        let j: JobStruct = hcl::from_str(job_str)?;
        dbg!(&j);

        let sources = call!(self.sources, |port| SourcesMsg::Get(j.name, port))?;
        let job = match j.jtype {
            JobType::Fetch => {
                todo!()
            }
            JobType::Read => {
                todo!()
            }
            JobType::Stream => {
                todo!()
            }
        };
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_job_fetch() {
        let input = r#"
            name = "Fetch Test Job"
            type = "fetch"
            source = "test_source"
            output = "test_output.csv"
        "#;

        let mut e = Engine::new().await;
        let parsed_job = e.parse(input).expect("Failed to parse job");

        assert_eq!(parsed_job.name, "Fetch Test Job");
        assert!(matches!(parsed_job.jtype, JobType::Fetch));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }

    #[tokio::test]
    async fn test_parse_job_read() {
        let input = r#"
            name = "Read Test Job"
            type = "read"
            source = "test_source"
            output = "test_output.csv"
        "#;

        let mut e = Engine::new().await;
        let parsed_job = e.parse(input).expect("Failed to parse job");

        assert_eq!(parsed_job.name, "Read Test Job");
        assert!(matches!(parsed_job.jtype, JobType::Read));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }

    #[tokio::test]
    async fn test_parse_job_stream() {
        let input = r#"
            name = "Stream Test Job"
            type = "stream"
            source = "test_source"
            output = "test_output.csv"
        "#;

        let mut e = Engine::new().await;
        let parsed_job = e.parse(input).expect("Failed to parse job");

        assert_eq!(parsed_job.name, "Stream Test Job");
        assert!(matches!(parsed_job.jtype, JobType::Stream));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }

    #[tokio::test]
    async fn test_parse_job_stream_split() {
        let input = r#"
            name = "Stream Test Job"
            type = "stream"
            source = "test_source"
            split = "/tmp/nowhere"
            output = "test_output.csv"
        "#;

        let mut e = Engine::new().await;
        let parsed_job = e.parse(input).expect("Failed to parse job");

        assert_eq!(parsed_job.name, "Stream Test Job");
        assert!(matches!(parsed_job.jtype, JobType::Stream));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
        assert_eq!(parsed_job.split, Some("/tmp/nowhere".to_string()));
    }
}
