//! This is the `fetiched` daemon launcher
//!
//! It could have been part of `acutectl`  but it is cleaner that way.
//!

use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use actix::prelude::*;
use clap::Parser;
use eyre::Result;
use log::error;
use tokio::fs;
use tokio::time::sleep;
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use fetiched::{
    ConfigActor, ConfigList, ConfigSet, EngineActor, EngineStatus, EngineVersion, Param,
};

use crate::cli::Opts;

mod cli;

/// Daemon name
const NAME: &str = env!("CARGO_BIN_NAME");

/// Daemon version
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[actix_rt::main]
async fn main() -> Result<()> {
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

    let pid_file = PathBuf::from("/tmp/fetiched.pid");
    if pid_file.exists() {
        info!("PID exist");
        let pid = fs::read_to_string(&pid_file)
            .await?
            .trim_end()
            .parse::<u32>()?;
        eprintln!("Check PID {}", pid);
        std::process::exit(1);
    }

    if opts.debug {
        info!("Debug mode, no detaching.");
        let pid = std::process::id();
        fs::write(&pid_file, &format!("{pid}")).await?;
    } else {
        #[cfg(unix)]
        start_daemon(&pid_file);
    }

    trace!("Starting configuration agent");
    let config = ConfigActor::default().start();

    trace!("Starting engine agent");
    let engine = EngineActor::default().start();

    let r = match engine.send(EngineVersion {}).await {
        Ok(res) => res,
        Err(e) => {
            error!("dead actor: {}", e.to_string());
            e.to_string()
        }
    };

    let _ = config
        .send(ConfigSet {
            name: "fetiche".to_string(),
            value: Param::String(r),
        })
        .await?;
    config.do_send(ConfigList {});
    engine.do_send(EngineStatus {});

    trace!("Init done, serving.");

    sleep(Duration::from_secs(10)).await;

    trace!("Finished.");
    fs::remove_file(&pid_file).await?;
    Ok(())
}

/// UNIX-specific detach from terminal if -D/--debug is not specified
///
#[cfg(unix)]
fn start_daemon(pid: &PathBuf) -> Result<()> {
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

            let stdout = io::stdout();

            info!("daemon is running");
        }
        Err(e) => error!("Error: {}", e),
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
