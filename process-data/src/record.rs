//! This contains the functions to record history in the database for runs.
//!

use chrono::{DateTime, Utc};

#[derive(Debug)]
#[repr(u8)]
pub enum RecordStatus {
    Correct = 0,
    NoPlanes,
    NoDrones,
    NoPotential,
    NoNewEncounters,
}

#[derive(Debug)]
pub struct Record {
    pub day: DateTime<Utc>,
    pub site_id: i32,
    pub site_name: String,
    pub status: RecordStatus,
    pub comment: String,
}
