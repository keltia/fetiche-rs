//! This is the module for data types for the `system_alert` / `dl_system_alert` queues
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Deserialize, strum::Display, EnumString, strum::VariantNames, Serialize)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Represents the data structure for a system alert.
///
/// This structure is used to encapsulate information about system alerts,  
/// including metadata, title, severity, timestamp, and detailed information.  
/// It serves as the data type for messages in the `system_alert` and
/// `dl_system_alert` queues.
///
#[derive(Debug, Deserialize, Serialize)]
pub struct AlertData {
    pub version: Option<String>,
    pub title: String,
    pub timestamp: DateTime<Utc>,
    pub severity: Severity,
    pub details: String,
}
