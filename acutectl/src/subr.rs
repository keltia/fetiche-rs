//! Miscellaneous subroutines.
//!
//! This is mostly for UNIX-specific stuff.
//!

use std::fs::File;
use std::path::{Path, PathBuf};
use tracing::{error, info};

/// UNIX-specific detach from terminal if -D/--debug is not specified
///
/// FIXME: using /tmp by default sucks.
///
#[cfg(unix)]
#[tracing::instrument]
pub fn start_daemon(base: &str, workdir: &PathBuf) -> eyre::Result<()> {
    let stdout = File::create(format!("/tmp/{}.out", base))?;
    let stderr = File::create(format!("/tmp/{}.err", base))?;

    let pid = Path::new(workdir).join("pid");
    let daemon = daemonize::Daemonize::new()
        .pid_file(&workdir)
        .working_directory("/tmp")
        .umask(0o077)
        .stdout(stdout)
        .stderr(stderr);

    match daemon.start() {
        Ok(_) => {
            info!("In child, detached");

            info!("daemon is running");
        }
        Err(e) => {
            error!("Error: {}", e);
            return Err(e.into());
        }
    }
    Ok(())
}
