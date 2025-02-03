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
    #[serde(rename = "type")]
    pub jtype: JobType,
    pub name: String,
    pub source: String,
    pub tee: Option<String>,
    pub split: Option<String>,
    pub save: Option<String>,
    pub output: String,
}

impl JobStruct {
    pub fn parse(job_str: &str) -> Result<JobStruct> {
        let j: JobStruct = hcl::from_str(job_str)?;
        dbg!(&j);
        Ok(j)
    }
}

fn main() {}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_job_fetch() {
        let input = r#"
            jobstruct {
                name = "Fetch Test JobStruct"
                type = "fetch"
                source = "test_source"
                output = "test_output.csv"
            }
        "#;

        let parsed_job = JobStruct::parse(input).expect("Failed to parse JobStruct");
        assert_eq!(parsed_job.name, "Fetch Test JobStruct");
        assert!(matches!(parsed_job.jtype, JobType::Fetch));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }

    #[test]
    fn test_parse_job_read() {
        let input = r#"
            jobstruct {
                name = "Read Test JobStruct"
                type = "read"
                source = "test_source"
                output = "test_output.csv"
            }
        "#;

        let parsed_job = JobStruct::parse(input).expect("Failed to parse JobStruct");
        assert_eq!(parsed_job.name, "Read Test JobStruct");
        assert!(matches!(parsed_job.jtype, JobType::Read));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }

    #[test]
    fn test_parse_job_stream() {
        let input = r#"
            jobstruct {
                name = "Stream Test JobStruct"
                type = "stream"
                source = "test_source"
                output = "test_output.csv"
            }
        "#;

        let parsed_job = JobStruct::parse(input).expect("Failed to parse JobStruct");
        assert_eq!(parsed_job.name, "Stream Test JobStruct");
        assert!(matches!(parsed_job.jtype, JobType::Stream));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }
}


