use std::collections::BTreeMap;

use serde::Deserialize;

use fetiche_common::{IntoConfig, Versioned};
use fetiche_macros::into_configfile;
use fetiche_sources::Auth;

/// Config filename
const CONFIG: &str = "config.hcl";
/// Current version
pub const CVERSION: usize = 1;

/// Configuration for the CLI tool, supposed to include parameters and most importantly
/// credentials for the various sources.
///
#[into_configfile]
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Each site credentials
    pub site: BTreeMap<String, Auth>,
}
