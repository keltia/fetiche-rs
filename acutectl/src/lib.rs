//! Library part of the `acutectl` utility.
//!
//! This library include the code for the different file formats used as input and the different
//! way of fetching data from different sources.  This is written because there are as many ways
//! to authenticate and connect as there are sources more or less.
//!
//! The `client` crate is now used for creating and submitting a job.
//!

/// Re-export
///
pub use cli::*;
pub use cmds::*;
pub use error::*;
#[cfg(unix)]
pub use subr::*;

mod cli;
mod cmds;
mod error;
#[cfg(unix)]
mod subr;

#[cfg(test)]
mod tests {
    use fetiche_common::{ConfigFile, IntoConfig, Versioned};
    use fetiche_macros::into_configfile;
    use serde::Deserialize;

    /// Configuration for the CLI tool, supposed to include parameters and most importantly
    /// credentials for the various sources.
    ///
    #[into_configfile]
    #[derive(Debug, Default, Deserialize)]
    struct Bar {
        /// Each site credentials
        pub use_async: bool,
    }

    pub const CVERSION: usize = 2;

    #[test]
    fn test_config_engine_load_default() -> eyre::Result<()> {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/acutectl.hcl");
        let cfg = ConfigFile::<Bar>::load(Some(path))?;
        let inner = cfg.inner();
        assert_eq!(CVERSION, inner.version());
        assert!(!inner.use_async);
        Ok(())
    }
}
