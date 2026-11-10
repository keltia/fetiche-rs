use thiserror::Error;

use crate::cmds::CmdError;

#[allow(dead_code)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Status {
    #[error("No database specified anywhere (config: {0})")]
    NoDatabase(String),
    #[error("No datalake specified in {0}")]
    NoDatalake(String),
    #[error("Can't get a connection from pool {0}")]
    ConnectionUnavailable(String),
    #[error("No database URL specified in {0}")]
    NoUrl(String),
    #[error("Bad file version {0}")]
    BadFileVersion(usize),
    #[error("Missing configuration file, use -d or create {0}")]
    MissingConfig(String),
    #[error("Error reading configuration({0})")]
    MissingConfigParameter(String),
    #[error("Missing parameter {0} in profile {1}")]
    MissingProfileParameter(String, String),
    #[error("Missing profile {0}")]
    MissingProfile(String),
    #[error("Missing airports path in {0}")]
    MissingAirportsFile(String),
    #[error(transparent)]
    Cmd(#[from] CmdError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_display() {
        let err = Status::NoDatabase("config.hcl".into());
        assert_eq!(
            err.to_string(),
            "No database specified anywhere (config: config.hcl)"
        );

        let err = Status::BadFileVersion(2);
        assert_eq!(err.to_string(), "Bad file version 2");

        let err = Status::MissingProfile("dev".into());
        assert_eq!(err.to_string(), "Missing profile dev");
    }

    #[test]
    fn test_status_from_cmderror() {
        let cmd_err = CmdError::BadDateFormat("invalid-date".into());
        let status: Status = cmd_err.into();
        assert_eq!(
            status,
            Status::Cmd(CmdError::BadDateFormat("invalid-date".into()))
        );
        assert_eq!(status.to_string(), "Unknown date format invalid-date.");
    }
}
