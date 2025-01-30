//! Definition of our job queue.
//!
use std::collections::VecDeque;

use crate::Job;

/// Representation of a job queue using a `VecDeque` to store jobs.
///
/// This struct provides a simple queue implementation for storing
/// and managing `Job` instances.
///
/// # Examples
///
/// ```
/// use fetiche_engine::{Job, JobQueue};
///
/// let mut queue = JobQueue::new();
///
/// let job = Job::new("Example Task");
/// queue.add(job);
///
/// if let Some(retrieved_job) = queue.get(0) {
///     println!("Retrieved Job: {:?}", retrieved_job);
/// }
/// ```
///
#[derive(Debug, Default)]
pub struct JobQueue(VecDeque<Job>);

impl JobQueue {
    #[tracing::instrument]
    pub fn new() -> Self {
        JobQueue::default()
    }

    #[tracing::instrument(skip(self))]
    pub fn get(&self, id: usize) -> Option<&Job> {
        self.0.get(id)
    }

    #[tracing::instrument(skip(self))]
    pub fn add(&mut self, job: Job) -> &mut Self {
        self.0.push_back(job);
        self
    }

    #[tracing::instrument(skip(self))]
    pub fn list(&self) -> Vec<usize> {
        self.0.iter().map(|j| j.id).collect::<Vec<usize>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Job;

    #[test]
    fn test_create_new_queue() {
        let queue = JobQueue::new();
        assert!(queue.0.is_empty());
    }

    #[test]
    fn test_add_job_to_queue() {
        let mut queue = JobQueue::new();
        let job = Job::new("Test Job");
        queue.add(job);

        assert_eq!(queue.0.len(), 1);
    }

    #[test]
    fn test_retrieve_job_from_queue() {
        let mut queue = JobQueue::new();
        let job = Job::new("Test Job");
        queue.add(job);

        let retrieved_job = queue.get(0);
        assert!(retrieved_job.is_some());
        assert_eq!(retrieved_job.unwrap().name, "Test Job");
    }

    #[test]
    fn test_get_nonexistent_job() {
        let queue = JobQueue::new();

        let retrieved_job = queue.get(0);
        assert!(retrieved_job.is_none());
    }
}
