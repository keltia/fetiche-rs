use std::collections::BTreeMap;

#[cfg(unix)]
use home::home_dir;
use serde::Deserialize;

use fetiche_common::Versioned;
use fetiche_sources::Auth;

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Config filename
const CONFIG: &str = "config.hcl";
/// Current version
pub const CVERSION: usize = 1;

/// Configuration for the CLI tool, supposed to include parameters and most importantly
/// credentials for the various sources.
///
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Version in the file MUST match `CVERSION`
    pub version: usize,
    /// Each site credentials
    pub site: BTreeMap<String, Auth>,
}

impl Versioned for Config {
    fn version(&self) -> usize {
        CVERSION
    }
}
