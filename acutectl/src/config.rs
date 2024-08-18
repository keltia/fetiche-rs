use std::collections::BTreeMap;

use serde::Deserialize;

use fetiche_common::Versioned;
use fetiche_macros::add_version;
use fetiche_sources::Auth;

/// Config filename
const CONFIG: &str = "config.hcl";
/// Current version
pub const CVERSION: usize = 1;

/// Configuration for the CLI tool, supposed to include parameters and most importantly
/// credentials for the various sources.
///
#[add_version(1)]
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// Each site credentials
    pub site: BTreeMap<String, Auth>,
}
