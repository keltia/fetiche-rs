use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum CmdError {
    #[error("Unknown date format {0}.")]
    BadDateFormat(String),
    #[error("Invalid encounter ID {0}.")]
    BadEncounterID(String),
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
}
