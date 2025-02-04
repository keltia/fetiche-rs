use eyre::Result;
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};

#[derive(Clone, Debug, Deserialize, EnumString, Serialize, PartialEq, VariantNames)]
#[serde(rename_all = "lowercase")]
pub enum JobType {
    Fetch,
    Read,
    Stream,
}

#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
pub enum Filter {
    Copy,
    Convert(String),
    Tee(String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum Consumer {
    Archive(String),
    Store(String, Freq),
    Save(String),
}

#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
pub enum Freq {
    Daily,
    Hourly,
}

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

impl JobText {
    pub fn parse(job_str: &str) -> Result<JobText> {
        let j: JobText = hcl::from_str(job_str)?;
        dbg!(&j);
        Ok(j)
    }
}

fn main() -> Result<()> {
    let j1 = JobText {
        jtype: JobType::Fetch,
        name: "Fetch Test JobText".to_string(),
        source: "test_source".to_string(),
        filters: Some(vec![Filter::Tee("test_tee".to_string()), Filter::Copy]),
        output: Consumer::Save("test_output.csv".to_string()),
    };
    let str = hcl::to_string(&j1)?;
    println!("{str}");


    let j2 = JobText {
        jtype: JobType::Fetch,
        name: "Fetch Test JobText".to_string(),
        source: "test_source".to_string(),
        filters: Some(vec![Filter::Tee("test_tee".to_string()), Filter::Copy]),
        output: Consumer::Store("test_output.csv".to_string(), Freq::Daily),
    };
    let str = hcl::to_string(&j2)?;
    println!("{str}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_job_text_valid() {
        let input = r#"
    type = "fetch"
    name = "Test Job"
    source = "test_source"
    filters = [
        { Tee = "filter_tee" },
        "Copy",
    ]
    output = {
        Save = "output.csv"
    }
    "#;

        let parsed_job = JobText::parse(input);

        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.jtype, JobType::Fetch);
        assert_eq!(job.name, "Test Job");
        assert_eq!(job.source, "test_source");
        assert!(job.filters.is_some());
        let filters = job.filters.unwrap();
        assert_eq!(filters.len(), 2);
        assert!(matches!(filters[0], Filter::Tee(ref val) if val == "filter_tee"));
        assert!(matches!(filters[1], Filter::Copy));
        assert_eq!(job.output, Consumer::Save("output.csv".to_string()));
    }

    #[test]
    fn parse_job_text_with_no_filters() {
        let input = r#"
    type = "read"
    name = "Job Without Filters"
    source = "source_name"
    output = {
        Save = "no_filters.csv"
    }
    "#;

        let parsed_job = JobText::parse(input);

        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.jtype, JobType::Read);
        assert_eq!(job.name, "Job Without Filters");
        assert_eq!(job.source, "source_name");
        assert!(job.filters.is_none());
        assert_eq!(job.output, Consumer::Save("no_filters.csv".into()));
    }

    #[test]
    fn parse_job_text_with_store_daily() {
        let input = r#"
    type = "read"
    name = "Job Without Filters"
    source = "source_name"
    output = {
        "Store" = [
            "/tmp/somewhere",
            "Daily"
        ]
    }
    "#;

        let parsed_job = JobText::parse(input);

        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.jtype, JobType::Read);
        assert_eq!(job.name, "Job Without Filters");
        assert_eq!(job.source, "source_name");
        assert!(job.filters.is_none());
        assert_eq!(job.output, Consumer::Store("/tmp/somewhere".into(), Freq::Daily));
    }


    #[test]
    fn parse_job_text_invalid_type() {
        let input = r#"
    type = "invalid"
    name = "Invalid Job"
    source = "invalid_source"
    output = {
        Save = "output.csv"
    }
    "#;

        let parsed_job = JobText::parse(input);

        assert!(parsed_job.is_err());
    }

    #[test]
    fn parse_job_text_invalid_format() {
        let input = r#"
    job_type = "fetch"
    name = "Missing Required Field"
    "#;

        let parsed_job = JobText::parse(input);

        assert!(parsed_job.is_err());
    }
}
