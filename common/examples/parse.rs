use eyre::Result;
use serde::{Deserialize, Serialize};
use strum::{EnumString, VariantNames};

#[derive(Clone, Debug, Deserialize, EnumString, Serialize, PartialEq, VariantNames)]
#[serde(rename_all = "lowercase")]
pub enum Producer {
    Fetch(String),
    Read(String),
    Stream(String),
}

#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Middle {
    Copy,
    Convert(String),
    Tee(String),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Consumer {
    Archive(String),
    Store(String, Freq),
    Save(String),
}

#[derive(Clone, Debug, Deserialize, EnumString, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Freq {
    Daily,
    Hourly,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobText {
    /// Job name.
    pub name: String,
    /// Generator
    pub producer: Producer,
    /// Optional list of filters like `Tee` or `Save`.
    pub filters: Option<Vec<Middle>>,
    /// Output file name.
    pub output: Consumer,
}

impl JobText {
    pub fn parse(job_str: &str) -> Result<JobText> {
        let j: JobText = hcl::from_str(job_str)?;
        println!("--\n{:?}\n--", j);
        Ok(j)
    }
}

#[derive(Clone, Debug)]
pub enum Task {
    /// Producer task that generates or sources data
    Producer(Producer),
    /// Middle task that transforms or processes data
    Middle(Middle),
    /// Consumer task that consumes or stores the final data
    Consumer(Consumer),
}

fn main() -> Result<()> {
    let j1 = JobText {
        producer: Producer::Fetch("test_source".to_string()),
        name: "Test Job JobText".to_string(),
        filters: Some(vec![Middle::Tee("test_tee".to_string()), Middle::Copy]),
        output: Consumer::Save("test_output.csv".to_string()),
    };
    let str = hcl::to_string(&j1)?;
    println!("--\n{str}--");


    let j2 = JobText {
        producer: Producer::Fetch("test_source".to_string()),
        name: "Fetch Test JobText".to_string(),
        filters: Some(vec![Middle::Tee("test_tee".to_string()), Middle::Copy]),
        output: Consumer::Store("test_output.csv".to_string(), Freq::Daily),
    };
    let str = hcl::to_string(&j2)?;
    println!("--\n{str}--");

    let j3 = JobText {
        producer: Producer::Fetch("test_source".to_string()),
        name: "Fetch Test JobText".to_string(),
        filters: None,
        output: Consumer::Store("test_output.csv".to_string(), Freq::Daily),
    };

    let str = hcl::to_string(&j3)?;
    println!("--\n{str}--");

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_job_text_valid() {
        let input = r#"
name = "Test Job JobText"
producer = {
  fetch = "test_source"
}
filters = [
  {
    tee = "test_tee"
  },
  "copy"
]
output = {
  save = "test_output.csv"
}    "#;

        let parsed_job = JobText::parse(input);
        dbg!(&parsed_job);

        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.producer, Producer::Fetch("test_source".into()));
        assert_eq!(job.name, "Test Job JobText");
        assert!(job.filters.is_some());
        let filters = job.filters.unwrap();
        assert_eq!(filters.len(), 2);
        assert!(matches!(filters[0], Middle::Tee(ref val) if val == "test_tee"));
        assert!(matches!(filters[1], Middle::Copy));
        assert_eq!(job.output, Consumer::Save("test_output.csv".to_string()));
    }

    #[test]
    fn parse_job_text_with_no_filters() {
        let input = r#"
    name = "Job Without Filters"
    producer = {
        "read" = "source_name"
    }
    output = {
        save = "no_filters.csv"
    }
    "#;

        let parsed_job = JobText::parse(input);
        dbg!(&parsed_job);

        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.producer, Producer::Read("source_name".into()));
        assert_eq!(job.name, "Job Without Filters");
        assert!(job.filters.is_none());
        assert_eq!(job.output, Consumer::Save("no_filters.csv".into()));
    }

    #[test]
    fn parse_job_text_with_store_daily() {
        let input = r#"
    name = "Job Without Filters"
    producer = {
        "read" = "source_name"
    }
    filters = null
    output = {
        store = [
            "/tmp/somewhere",
            "daily"
        ]
    }
    "#;

        let parsed_job = JobText::parse(input);
        dbg!(&parsed_job);

        assert!(parsed_job.is_ok());
        let job = parsed_job.unwrap();

        assert_eq!(job.producer, Producer::Read("source_name".into()));
        assert_eq!(job.name, "Job Without Filters");
        assert!(job.filters.is_none());
        assert_eq!(job.output, Consumer::Store("/tmp/somewhere".into(), Freq::Daily));
    }


    #[test]
    fn parse_job_text_invalid_type() {
        let input = r#"
    producer = "invalid"
    name = "Invalid Job"
    source = "invalid_source"
    output = {
        save = "output.csv"
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
