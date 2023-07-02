use std::collections::BTreeMap;
use std::path::PathBuf;

/// We define a `Store` enum, describing storage areas like a directory or an S3
/// bucket (from an actual AWS account or a Garage instance).
///
#[derive(Clone, Debug)]
pub enum StoreArea {
    /// S3 AWS/Garage bucket
    Bucket { name: String, region: String },
    /// in-memory K/V store like DragonflyDB or REDIS
    Cache { url: String },
    /// In the local filesystem
    Directory { path: PathBuf },
}

pub struct StorageAreas(BTreeMap<String, StoreArea>);

impl StoreArea {
    pub fn save(path: &str) -> Self {
        Self::Directory {
            path: PathBuf::from(path),
        }
    }

    pub fn send(region: &str, bucket: &str) -> Self {
        Self::Bucket {
            region: region.to_owned(),
            name: bucket.to_owned(),
        }
    }
}
