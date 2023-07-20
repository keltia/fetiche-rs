//! This is the `fetiched` daemon launcher
//!
//! It could have been part of `acutectl`  but it is cleaner that way.
//!

use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, io};

use anyhow::Result;
use clap::Parser;
use tracing::info;
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

    let daemon = daemonize::Daemonize::new()
        .pid_file(&pid)
        .working_directory("/tmp")
        .stdout(stdout)
        .stderr(stderr);

    match daemon.start() {
        Ok(_) => {
            info!("In child, detached");

            let mut stdout = io::stdout();

            info!("sleep");
            std::thread::sleep(Duration::from_secs(60));
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    Ok(fs::remove_file(&pid)?)
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
