//! Local Engine client library for Fetiche.
//!
//! # Overview
//!
//! The client library provides the core data structures and types for defining and configuring
//! data processing jobs in the Fetiche engine. It acts as an interface layer between the user's
//! job definitions and the engine's execution environment.
//!
//! This is front-end API.
//!
//! The `JobBuilder` provides a fluent interface for constructing data processing jobs.
//! It allows configuring various aspects of a job including data sources (fetch or stream),
//! filters, middleware operations (tee), and output destinations (save or store).
//!
use eyre::Result;

use crate::{ConsumerText, Freq, JobText, JobTextBuilder, MiddleText, ProducerText};

use fetiche_engine::{Filter, ParserError};

#[derive(Clone, Debug, Default)]
pub enum JobType {
    Fetch(String),
    Stream(String),
    #[default]
    Invalid,
}

#[derive(Debug, Clone, Default)]
/// A builder for constructing data processing jobs with a fluent interface.
///
/// `JobBuilder` provides methods to configure various aspects of a job:
/// * Job identification through a name
/// * Data source configuration (fetch or stream)
/// * Data filtering capabilities
/// * Middleware operations like tee
/// * Output handling (save or store)
///
/// All configuration is done through method chaining, with the final job
/// being created by calling `build()`.
///
pub struct JobBuilder {
    /// Job description
    name: String,
    /// This is the job type with the embedded site name
    producer: JobType,
    /// Filter to apply to the data
    filter: Option<Filter>,
    /// Middleware operations like tee
    middle: Vec<MiddleText>,
    /// Output handling (save or store)
    output: Option<ConsumerText>,
}

impl JobBuilder {
    /// Creates a new JobBuilder with the specified name.
    ///
    /// # Arguments
    /// * `name` - The name of the job to be created
    ///
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Configures the job to fetch data from a specified site.
    ///
    /// # Arguments
    /// * `site` - The URL or identifier of the site to fetch from
    ///
    #[tracing::instrument(skip(self))]
    pub fn fetch(&mut self, site: &str) -> &mut Self {
        self.producer = JobType::Fetch(site.to_string());
        self
    }

    /// Configures the job to stream data from a specified site.
    ///
    /// # Arguments
    /// * `site` - The URL or identifier of the site to stream from
    ///
    #[tracing::instrument(skip(self))]
    pub fn stream(&mut self, site: &str) -> &mut Self {
        self.producer = JobType::Stream(site.to_string());
        self
    }

    /// Adds a filter to the job for processing data.
    ///
    /// # Arguments
    /// * `f` - The filter to apply to the data
    ///
    #[tracing::instrument(skip(self))]
    pub fn filter(&mut self, f: Filter) -> &mut Self {
        self.filter = Some(f);
        self
    }

    /// Adds a tee operation to write data to a file while passing it through.
    ///
    /// # Arguments
    /// * `fname` - The name of the file to write to
    ///
    #[tracing::instrument(skip(self))]
    pub fn tee(&mut self, fname: Option<String>) -> &mut Self {
        if let Some(fname) = fname {
            self.middle.push(MiddleText::Tee(fname.clone()));
        }
        self
    }

    /// Configures the job to save its output to a file.
    ///
    /// # Arguments
    /// * `fname` - The name of the file to save to
    ///
    #[tracing::instrument(skip(self))]
    pub fn save(&mut self, fname: &str) -> &mut Self {
        self.output = Some(ConsumerText::Save(fname.to_string()));
        self
    }

    /// Configures the job to store its output with specified frequency.
    ///
    /// # Arguments
    /// * `path` - The path where data should be stored
    /// * `freq` - The frequency at which data should be stored
    ///
    #[tracing::instrument(skip(self))]
    pub fn store(&mut self, path: &str, freq: Freq) -> &mut Self {
        self.output = Some(ConsumerText::Store(path.to_string(), freq));
        self
    }

    /// Builds the final job configuration.
    ///
    #[tracing::instrument(skip(self))]
    pub fn build(&mut self) -> Result<JobText> {
        let producer = match &self.producer {
            JobType::Fetch(site) => ProducerText::Fetch(site.clone(), self.filter.clone().unwrap()),
            JobType::Stream(site) => {
                ProducerText::Stream(site.clone(), self.filter.clone().unwrap())
            }
            _ => return Err(ParserError::InvalidJobType.into()),
        };

        Ok(JobTextBuilder::default()
            .name(self.name.clone())
            .producer(producer)
            .middle(Some(self.middle.clone()))
            .output(self.output.clone().unwrap())
            .build()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fetiche_engine::Filter;

    #[test]
    fn test_new_job_builder() {
        let builder = JobBuilder::new("test_job");
        assert_eq!(builder.name, "test_job");
        assert!(matches!(builder.producer, JobType::Invalid));
    }

    #[test]
    fn test_fetch_job() -> Result<()> {
        let mut builder = JobBuilder::new("test_fetch");
        let filter = Filter::default();
        builder.fetch("somesite").filter(filter);
        let job = builder.save("file").build()?;
        assert_eq!(job.name, "test_fetch");
        assert!(matches!(job.producer, ProducerText::Fetch(_, _)));
        Ok(())
    }

    #[test]
    fn test_stream_job() -> Result<()> {
        let mut builder = JobBuilder::new("test_stream");
        let filter = Filter::default();
        builder.stream("anothersite").filter(filter);
        let job = builder.save("file").build()?;
        assert_eq!(job.name, "test_stream");
        assert!(matches!(job.producer, ProducerText::Stream(_, _)));
        Ok(())
    }

    #[test]
    fn test_tee_middleware() -> Result<()> {
        let mut builder = JobBuilder::new("test_tee");
        let filter = Filter::default();
        builder
            .fetch("https://example.com")
            .filter(filter)
            .tee(Some("output.txt".to_string()))
            .save("final.txt");
        let job = builder.build()?;
        assert_eq!(job.middle.clone().unwrap().len(), 1);
        assert!(matches!(job.middle.unwrap()[0], MiddleText::Tee(_)));
        Ok(())
    }

    #[test]
    fn test_save_output() -> Result<()> {
        let mut builder = JobBuilder::new("test_save");
        let filter = Filter::default();
        builder
            .fetch("https://example.com")
            .filter(filter)
            .save("output.txt");
        let job = builder.build()?;
        assert!(matches!(job.output, ConsumerText::Save(_)));
        Ok(())
    }

    #[test]
    fn test_store_output() -> Result<()> {
        let mut builder = JobBuilder::new("test_store");
        let filter = Filter::default();
        builder
            .fetch("https://example.com")
            .filter(filter)
            .store("data", Freq::Daily);
        let job = builder.build()?;
        assert!(matches!(job.output, ConsumerText::Store(_, _)));
        Ok(())
    }
}
