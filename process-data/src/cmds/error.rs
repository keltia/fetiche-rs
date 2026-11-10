use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum CmdError {
    #[error("Unknown date format {0}.")]
    BadDateFormat(String),
    #[error("Invalid encounter ID {0}.")]
    BadEncounterID(String),
    #[error("Encounter mismatch: expected {expected}, got {found}.")]
    EncounterMismatch { expected: String, found: String },
    #[error("Journey mismatch: expected {expected}, got {found}.")]
    JourneyMismatch { expected: i32, found: i32 },
    #[error("Can't get a connection from pool {0}")]
    ConnectionUnavailable(String),
    #[error("Invalid site name {0}")]
    UnknownSite(String),
    #[error("Invalid site id {0}")]
    UnknownSiteId(u32),
    #[error("Either -A or a date, not both!")]
    NoAllAndDate,
    #[error("Either -A or --id, not both!")]
    NoAllAndENID,
    #[error("No encounter ID specified.")]
    NoEncounterSpecified,
    #[error("Not enough {0} data for a trajectory.")]
    NotEnoughData(String),
    #[error("No output destination specified, aborting.")]
    NoOutputDestination,
    #[error("{0}: Not a directory!.")]
    NotADirectory(String),
    #[error("Unknown output format, aborting.")]
    UnknownFormat(String),
    #[error("Timestamp out of range {0}")]
    BadTimestamp(String),
    #[error("Bad filename pattern {0}")]
    BadFilenamePattern(String),
    #[error("Need a file in CSV format {0}")]
    NeedCsvFile(String),
    #[error("Force needed with \"-f\" or \"--force\".")]
    ForceNeeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encounter_mismatch_display() {
        let err = CmdError::EncounterMismatch {
            expected: "ENC-001".into(),
            found: "ENC-002".into(),
        };
        assert_eq!(
            err.to_string(),
            "Encounter mismatch: expected ENC-001, got ENC-002."
        );
    }

    #[test]
    fn test_journey_mismatch_display() {
        let err = CmdError::JourneyMismatch {
            expected: 42,
            found: 100,
        };
        assert_eq!(
            err.to_string(),
            "Journey mismatch: expected 42, got 100."
        );
    }

    #[test]
    fn test_not_a_directory_display() {
        let err = CmdError::NotADirectory("/tmp/foo.txt".into());
        assert_eq!(err.to_string(), "/tmp/foo.txt: Not a directory!.");
    }
}
