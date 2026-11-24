//! This is the module for data types for the `system_alert` / `dl_system_alert` queues
//!

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::EnumString;

use crate::common::Rfc3339Timestamp;

#[derive(Clone, Debug, PartialEq, Deserialize, strum::Display, EnumString, strum::VariantNames, Serialize)]
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
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct AlertData {
    pub version: Option<String>,
    pub title: String,
    #[serde_as(serialize_as = "Rfc3339Timestamp")]
    pub timestamp: Timestamp,
    pub severity: Severity,
    pub details: String,
}
