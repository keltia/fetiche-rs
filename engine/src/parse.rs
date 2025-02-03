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

use crate::actors::SourcesMsg;
use crate::Engine;
use eyre::Result;
use ractor::call;
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Clone, Debug, Deserialize, EnumString, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    Fetch,
    Read,
    Stream,
}

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
