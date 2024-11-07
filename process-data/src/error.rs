use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum Status {
    #[error("Invalid site name {0}")]
    UnknownSite(String),
    #[error("No database specified anywhere (config: {0}")]
    NoDatabase(String),
    #[error("No datalake specified in {0}")]
    NoDatalake(String),
    #[error("No database URL specified in {0}")]
    NoUrl(String),
    #[error("Bad file version {0}")]
    BadFileVersion(usize),
    #[error("Missing configuration file, use -d or create {0}")]
    MissingConfig(String),
    #[error("Error reading configuration({0})")]
    MissingConfigParameter(String),
    #[error("Can't get a connection from pool {0}")]
    ConnectionUnavailable(String),
    #[error("No output destination specified, aborting.")]
    NoOutputDestination,
    #[error("Unknown output format, aborting.")]
    UnknownFormat(String),
    #[error("Unknown date format {0}.")]
    BadDateFormat(String),
    #[error("Invalid encounter ID {0}.")]
    BadEncounterID(String),
    #[error("Not enough {0} data for a trajectory.")]
    NotEnoughData(String),
    #[error("{0}: Not a directory!.")]
    NotADirectory(String),
    #[error("Either -A or a date, not both!")]
    NoAllAndDate,
    #[error("Either -A or --id, not both!")]
    NoAllAndENID,
    #[error("No encounter ID specified.")]
    NoEncounterSpecified,
}
