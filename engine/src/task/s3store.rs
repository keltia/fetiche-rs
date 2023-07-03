//! This is the task that store value obtained through the input pipe
//! into a S3 bucket.
//!
//! S3 bucket can be some any AWS S3 compatible storage like Garage or Minio
//!

use crate::IO;

#[derive(Clone, Debug, RunnableDerive)]
pub struct S3store {
    /// I/O capability
    io: IO,
    /// S3 Bucket ID
    bucket: String,
}

impl Default for S3store {
    fn default() -> Self {
        S3store {
            io: IO::Consumer,
            bucket: "".to_string(),
        }
    }
}

impl S3store {
    pub fn new(bucket: String) -> Self {
        let mut s = S3store::default();
        s.bucket = bucket.clone();
        s
    }
}
