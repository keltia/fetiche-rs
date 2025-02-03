use eyre::Result;
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};

#[derive(Clone, Debug, Deserialize, EnumString, Serialize, VariantNames)]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    Fetch,
    Read,
    Stream,
}

#[derive(Debug, Serialize, Deserialize)]
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

fn main() -> Result<()> {
    let j1 = JobStruct {
        jtype: JobType::Fetch,
        name: "Fetch Test JobStruct".to_string(),
        source: "test_source".to_string(),
        tee: Some("test_tee".to_string()),
        split: Some("test_split".to_string()),
        save: Some("test_save".to_string()),
        output: "test_output.csv".to_string(),
    };
    let str = hcl::to_string(&j1)?;
    println!("{str}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_job_fetch() {
        let input = r#"
                name = "Fetch Test JobStruct"
                type = "fetch"
                source = "test_source"
                output = "test_output.csv"
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
                name = "Read Test JobStruct"
                type = "read"
                source = "test_source"
                output = "test_output.csv"
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
                name = "Stream Test JobStruct"
                type = "stream"
                source = "test_source"
                output = "test_output.csv"
        "#;

        let parsed_job = JobStruct::parse(input).expect("Failed to parse JobStruct");
        assert_eq!(parsed_job.name, "Stream Test JobStruct");
        assert!(matches!(parsed_job.jtype, JobType::Stream));
        assert_eq!(parsed_job.source, "test_source");
        assert_eq!(parsed_job.output, "test_output.csv");
    }
}
