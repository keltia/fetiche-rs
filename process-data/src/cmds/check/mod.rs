//! `check` command module
//!

use clap::Parser;
use fetiche_common::DateOpts;

#[derive(Debug, Parser)]
pub struct CheckOpts {
    /// Check on a given day.
    #[clap(subcommand)]
    day: Option<DateOpts>,
    /// Check for a given site.
    site: Option<String>,
}
