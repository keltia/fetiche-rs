//! Definition of our job queue.
//!
use std::collections::VecDeque;

use tracing::warn;

use crate::Job;

/// This is the job queue with all defined jobs
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
}
