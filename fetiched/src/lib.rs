pub use actors::*;

mod actors;

/// Main state data file, will be created in `basedir`.
pub(crate) const STATE_FILE: &str = "state";

pub(crate) const DEF_HOMEDIR: &str = "/var/run/fetiche";
