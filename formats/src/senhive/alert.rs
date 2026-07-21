//! This is the module for data types for the `system_alert` / `dl_system_alert` queues
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[cfg(feature = "rkyv")]
use fetiche_macros::RkyvClone;

#[derive(Clone, Debug, PartialEq, Deserialize, strum::Display, EnumString, strum::VariantNames, Serialize)]
#[strum(serialize_all = "UPPERCASE")]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
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
#[cfg_attr(feature = "rkyv", derive(RkyvClone))]
#[derive(Debug, Deserialize, Serialize)]
pub struct AlertData {
    pub version: Option<String>,
    pub title: String,
    pub timestamp: DateTime<Utc>,
    pub severity: Severity,
    pub details: String,
}

#[cfg(all(test, feature = "rkyv"))]
mod tests {
    use super::*;

    #[test]
    fn test_rkyv_alert_data_roundtrip() {
        let alert = AlertData {
            version: Some("1.0.0".to_string()),
            title: "Test Alert".to_string(),
            timestamp: DateTime::from_timestamp(1234567890, 0).unwrap(),
            severity: Severity::Warning,
            details: "This is a test alert".to_string(),
        };

        // Convert to rkyv version
        let rkyv_alert: RAlertData = (&alert).into();

        // Verify timestamp conversion
        assert_eq!(rkyv_alert.timestamp_millis, 1234567890000);
        assert_eq!(rkyv_alert.title, "Test Alert");
        assert_eq!(rkyv_alert.severity, Severity::Warning);

        // Serialize with rkyv
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&rkyv_alert).unwrap();

        // Deserialize
        let decoded = rkyv::from_bytes::<RAlertData, rkyv::rancor::Error>(&bytes).unwrap();

        assert_eq!(decoded, rkyv_alert);

        // Convert back to original
        let recovered: AlertData = (&decoded).into();
        assert_eq!(recovered.title, alert.title);
        assert_eq!(recovered.severity, alert.severity);
        assert_eq!(recovered.timestamp.timestamp_millis(), alert.timestamp.timestamp_millis());
    }
}
