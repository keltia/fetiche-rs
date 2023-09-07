//! This is the `fetiched` daemon launcher
//!
//! It could have been part of `acutectl`  but it is cleaner that way.
//!

use std::fs::File;
use std::path::PathBuf;
use std::{fs, io};

use clap::Parser;
use eyre::Result;
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter::EnvFilter, fmt};

use crate::cli::Opts;

mod cli;

/// Daemon name
const NAME: &str = env!("CARGO_BIN_NAME");

/// Daemon version
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let fmt = fmt::layer()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(false)
        .compact();

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry().with(filter).with(fmt).init();

    let pid = PathBuf::from("/tmp/fetiched.pid");
    if pid.exists() {
        info!("PID exist");
        let pid = fs::read_to_string(&pid)?.trim_end().parse::<u32>()?;
        eprintln!("Check PID {}", pid);
        std::process::exit(1);
    }

    let stdout = File::create("/tmp/fetiched.out")?;
    let stderr = File::create("/tmp/fetiched.err")?;

    #[cfg(unix)]
    start_daemon(&pid, stdout, stderr);

    // Now start serving
    trace!("Serving.");

    trace!("Finished.");
    Ok(fs::remove_file(&pid)?)
}

#[cfg(unix)]
fn start_daemon(pid: &PathBuf, out: File, err: File) -> Result<()> {
    let daemon = daemonize::Daemonize::new()
        .pid_file(&pid)
        .working_directory("/tmp")
        .stdout(out)
        .stderr(err);

    match daemon.start() {
        Ok(_) => {
            info!("In child, detached");

            let stdout = io::stdout();

            info!("daemon is running");
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    Ok(())
}

/// Announce ourselves
pub(crate) fn version() -> String {
    format!(
        "{}/{} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        fetiche_engine::version(),
    )
}
