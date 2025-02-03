//! Compiler for the Fetiche job language
//!
//! Description of the job & task language -- leveraging HCL support
//!
//! ```hcl
//! job {
//!     name = "Opensky"
//!     type = "fetch"
//!     source = opensky
//!     output = "foo.csv"
//! }
//! ```

use crate::Job;
use eyre::Result;
use serde::Deserialize;
use strum::EnumString;

#[derive(Clone, Debug, Deserialize, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum JobType {
    Fetch,
    Read,
    Stream,
}

#[derive(Debug, Deserialize)]
pub struct JobStruct {
    pub name: String,
    pub jtype: JobType,
    pub source: String,
    pub tee: Option<String>,
    pub split: Option<String>,
    pub save: Option<String>,
    pub output: String,
}

impl Job {
    pub fn parse(job_str: &str) -> Result<JobStruct> {
        let j: JobStruct = hcl::from_str(job_str)?;
        dbg!(&j);
        Ok(j)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_job_fetch() {
        let input = r#"
            job {
                name = "Fetch Test Job"
                type = "fetch"
                source = "test_source"
                output = "test_output.csv"
            }
        "#;

        let parsed_job = Job::parse(input).expect("Failed to parse job");
        assert_eq!(parsed_job.name, "Fetch Test Job");
        assert!(matches!(parsed_job.jtype, JobType::Fetch));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }

    #[test]
    fn test_parse_job_read() {
        let input = r#"
            job {
                name = "Read Test Job"
                type = "read"
                source = "test_source"
                output = "test_output.csv"
            }
        "#;

        let parsed_job = Job::parse(input).expect("Failed to parse job");
        assert_eq!(parsed_job.name, "Read Test Job");
        assert!(matches!(parsed_job.jtype, JobType::Read));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }

    #[test]
    fn test_parse_job_stream() {
        let input = r#"
            job {
                name = "Stream Test Job"
                type = "stream"
                source = "test_source"
                output = "test_output.csv"
            }
        "#;

        let parsed_job = Job::parse(input).expect("Failed to parse job");
        assert_eq!(parsed_job.name, "Stream Test Job");
        assert!(matches!(parsed_job.jtype, JobType::Stream));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }
}

