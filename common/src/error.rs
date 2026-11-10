//! Error types for the fetiche-common crate.

use eyre::Report;
use thiserror::Error;

/// Error types related to container definitions and operations.
#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("container format version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: usize, actual: usize },
    #[error("failed to parse container definitions: {0}")]
    Parse(#[from] hcl::Error),
}

/// Error types related to date parsing and calculus options.
#[derive(Debug, Error)]
pub enum ErrDateOpts {
    #[error("bad date: {0}")]
    BadDate(String),
    #[error("bad week number: must be 1 <= {0} <= 53.")]
    BadWeekNumber(i64),
    #[error("cannot get date from system")]
    CannotGetDate,
    #[error("failed to calculate date: {0}")]
    FailedCalculus(String),
}

impl From<Report> for ErrDateOpts {
    fn from(value: Report) -> Self {
        ErrDateOpts::BadDate(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_error_display() {
        let err = ContainerError::VersionMismatch {
            expected: 2,
            actual: 3,
        };
        assert_eq!(
            err.to_string(),
            "container format version mismatch: expected 2, got 3"
        );
    }

    #[test]
    fn test_err_date_opts_display() {
        assert_eq!(
            ErrDateOpts::BadWeekNumber(55).to_string(),
            "bad week number: must be 1 <= 55 <= 53."
        );
        assert_eq!(
            ErrDateOpts::BadDate("2023-13-01".into()).to_string(),
            "bad date: 2023-13-01"
        );
        assert_eq!(
            ErrDateOpts::CannotGetDate.to_string(),
            "cannot get date from system"
        );
        assert_eq!(
            ErrDateOpts::FailedCalculus("overflow".into()).to_string(),
            "failed to calculate date: overflow"
        );
    }

    #[test]
    fn test_err_date_opts_from_report() {
        let report = eyre::eyre!("custom error message");
        let err = ErrDateOpts::from(report);
        assert!(matches!(err, ErrDateOpts::BadDate(msg) if msg == "custom error message"));
    }
}
