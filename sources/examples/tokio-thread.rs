// Tokio-based worker/alarm threads
//
// We may be going full async, hang on Baby, we're for a ride!
//

use std::env::Args;
use std::io::{stderr, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread, time};

use anyhow::Result;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
#[cfg(unix)]
use tokio::signal::ctrl_c;
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;
use tokio::sync::mpsc;

// If 0, infinite wait, need SIGINT to sop
//
const SLEEP: u64 = 20;

async fn worker_thread(out: &mut dyn Write, d: u64) -> Result<()> {
    // Launch it!
    //
    writeln!(stderr(), "Starting stream loop")?;
    // For data
    let (tx, mut rx) = mpsc::channel(20);
    // For alarm
    let (tx1, mut rx1) = mpsc::channel(1);

    if d != 0 {
        // setup alarm
        //
        writeln!(stderr(), "setup alarm")?;
        tokio::spawn(async move {
            thread::sleep(Duration::from_secs(SLEEP));
            tx1.send("bing!").await;
            return;
        });
    }

    // setup ctrl-c handled
    //
    let mut sig = ctrl_c()?;

    // start working
    //
    writeln!(stderr(), "working...");
    tokio::spawn(async move {
        loop {
            thread::sleep(Duration::from_secs(2 as u64));
            tx.send(".").await;
        }
    });

    let mut output = String::new();

    writeln!(stderr(), "get data thread")?;
    loop {
        tokio::select! {
            Some(msg) = rx.recv() => output.push_str(msg),
            Some(msg) = rx1.recv() => {
                output.push_str(&format!("{}:{}", msg, "finished!"));
                break;
            },
            _ = sig.recv() => {
                writeln!(stderr(), "out!")?;
                break;
            }
        }
        writeln!(out, "{}", output)?;
        out.flush();
    }
    writeln!(out, "{}", output)?;
    out.flush();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut out = stderr();

    let args = env::args();

    worker_thread(&mut out, SLEEP).await?;

    println!("with sleeper, nothing is displayed");
    std::process::exit(0);
}
