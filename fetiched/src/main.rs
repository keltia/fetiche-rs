//! This is the `fetiched` daemon launcher
//!
//! It could have been part of `acutectl` but it is cleaner that way.
//!
//! NOTE: this is a fully async daemon... calling the rest of the fetiche framework
//!       which is completely sync.  Do not ask me how this works :)

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

use actix::prelude::*;
use clap::Parser;
use eyre::{eyre, Result};
use tokio::fs;
use tokio::time::sleep;
use tracing::error;
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use fetiched::{
    ConfigActor, ConfigKeys, ConfigList, ConfigSet, EngineActor, GetStatus, GetVersion, Param,
    StateActor, StorageActor, Submit,
};

use crate::cli::{Opts, SubCommand};
use crate::config::default_workdir;

mod cli;
mod config;

/// Daemon name
const NAME: &str = env!("CARGO_BIN_NAME");

/// Daemon version
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[actix_rt::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // Initialise logging early
    //
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
    trace!("Logging initialised.");

    info!("This is {} starting up…", version());

    let workdir = opts.workdir.unwrap_or(default_workdir()?);
    let pid_file = workdir.join(Path::new("fetiched.pid"));

    trace!("Working directory is {:?}", workdir);

    if pid_file.exists() {
        info!("PID exist");
        let pid = fs::read_to_string(&pid_file)
            .await?
            .trim_end()
            .parse::<u32>()?;
        return Err(eyre!("Check PID {}", pid));
    }

    info!("PID = {}", std::process::id());

    // Bail out early
    //
    if opts.subcmd == SubCommand::Version {
        eprintln!("{}", version());
        return Ok(());
    }

    if opts.debug {
        info!("Debug mode, no detaching.");
        let pid = std::process::id();
    } else {
        #[cfg(unix)]
        if let Err(err) = start_daemon(&pid_file) {
            panic!("Can not detach: {}", err.to_string());
        }
    }

    trace!("Starting configuration agent");
    let config = ConfigActor::default().start();

    trace!("Starting storage agent");
    let storage = StorageActor::new(&workdir).start();

    trace!("Starting state agent");
    let state = StateActor::new(&workdir).start();

    trace!("Starting engine agent");
    let engine = EngineActor::default().start();

    let r = match engine.send(GetVersion).await {
        Ok(res) => res,
        Err(e) => {
            error!("dead actor: {}", e.to_string());
            e.to_string()
        }
    };

    config.do_send(ConfigSet {
        name: "fetiche".to_string(),
        value: Param::String(r),
    });

    match config.send(ConfigList).await? {
        Ok(res) => eprintln!("Config:\n{}", res),
        Err(e) => error!("Can not read configuration: {}", e.to_string()),
    }

    let res = config.send(ConfigKeys).await?;
    match res {
        Ok(res) => eprintln!("All config keys={}", res.join(",")),
        Err(e) => error!("Error getting keys: {}", e.to_string()),
    };

    match engine.send(GetStatus).await {
        Ok(status) => {
            info!(
                "Engine is running, home is {}, {} jobs in queue.",
                status.home, status.jobs
            );
        }
        Err(e) => {
            error!("dead actor: {}", e.to_string());
        }
    };

    trace!("Init done, serving.");

    // ネコ = neko = cat
    //
    let job = Submit::new("message \"ネコ\"");

    trace!("job = {:?}", job);

    let res = engine.send(job).await;

    let res = match res {
        Ok(res) => res,
        Err(e) => {
            error!("Can not send: {}", e.to_string());
            std::process::exit(1);
        }
    };

    println!("Res = {}", res);

    assert_eq!("ネコ", res);

    sleep(Duration::from_secs(10)).await;

    trace!("Finished.");
    if !opts.debug {
        let _ = fs::remove_file(&pid_file).await;
    }
    System::current().stop();
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

/// Announce ourselves
pub(crate) fn version() -> String {
    format!("{}/{} ({})", NAME, VERSION, fetiche_engine::version(),)
}
