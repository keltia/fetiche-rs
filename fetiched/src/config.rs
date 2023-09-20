use std::path::PathBuf;

use eyre::Result;
use tracing::{info, trace};

/// Default working directory (UNIX)
#[cfg(unix)]
pub(crate) const DEF_HOMEDIR: &str = "/var/db/fetiche";

/// Returns the path of the default working directory file. On Unix systems we use `/var/db/fetiche`
/// as we want persistence.  `/var/run` is often a ramdisk or something cleaned up on reboot.
/// This can be overridden with environment variable `FETICHE_HOME`
///
#[cfg(unix)]
#[tracing::instrument]
pub(crate) fn default_workdir() -> Result<PathBuf> {
    trace!("Check for FETICHE_HOME var.");

    let def = match std::env::var("FETICHE_HOME") {
        Ok(path) => {
            info!("Will run from {path}");

            PathBuf::from(path)
        }
        _ => {
            let workdir = PathBuf::from(DEF_HOMEDIR);
            info!("Will run from {workdir:?}");

            // Create if not existing.
            //
            if !workdir.exists() {
                let _ = std::fs::create_dir_all(DEF_HOMEDIR)?;
            }
            workdir
        }
    };
    Ok(def)
}

/// Returns the path of the default config file.  Here we use the standard %LOCALAPPDATA%
/// variable to base our directory into.
///
#[cfg(windows)]
#[tracing::instrument]
pub(crate) fn default_workdir() -> Result<PathBuf> {
    trace!("Check for FETICHE_HOME var.");

    let workdir = match std::env::var("FETICHE_HOME") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            let local = std::env::var("LOCALAPPDATA")?;

            let workdir: PathBuf = [PathBuf::from(local), PathBuf::from("fetiche")]
                .iter()
                .collect();

            // Create if not existing.
            //
            if !workdir.exists() {
                let _ = std::fs::create_dir_all(&workdir)?;
            }
            workdir
        }
    };
    Ok(workdir)
}
