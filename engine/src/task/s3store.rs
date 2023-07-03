//! This is the task that store value obtained through the input pipe
//! into a S3 bucket.
//!
//! S3 bucket can be some any AWS S3 compatible storage like Garage or Minio
//!

use std::sync::mpsc::Sender;

use anyhow::Result;

use engine_macros::RunnableDerive;

use crate::{Runnable, IO};

#[derive(Clone, Debug, RunnableDerive)]
pub struct S3store {
    /// I/O capability
    io: IO,
    /// S3 Bucket ID
    bucket: String,
    /// AWS key
    key: String,
    /// AWS secret
    secret: String,
}

impl Default for S3store {
    fn default() -> Self {
        S3store::new()
    }
}

impl S3store {
    pub fn new() -> Self {
        S3store {
            io: IO::Consumer,
            bucket: "".to_string(),
            key: "".to_string(),
            secret: "".to_string(),
        }
    }

    pub fn region(&mut self, region: String) -> &mut Self {
        self.region = region
        self
    }

    pub fn info(&mut self, bucket: String) -> &mut Self {
        self.bucket = bucket
        self
    }

    pub fn with(&mut self, )
    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        Ok(())
    }
}
