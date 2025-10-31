//! Miscellaneous subroutines.
//!
//! This is mostly for UNIX-specific stuff.
//!

/// UNIX-specific detach from terminal if -D/--debug is not specified
///
#[cfg(unix)]
#[tracing::instrument]
pub fn start_daemon(pid: &PathBuf) -> eyre::Result<()> {
    let stdout = File::create(format!("/tmp/{NAME}.out"))?;
    let stderr = File::create(format!("/tmp/{NAME}.err"))?;

    let daemon = daemonize::Daemonize::new()
        .pid_file(&pid)
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
