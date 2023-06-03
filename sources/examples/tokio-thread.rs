// Tokio-based worker/alarm threads
//
// We may be going full async, hang on Baby, we're for a ride!
//

use std::io::{stderr, Write};
use std::thread;
use std::time::Duration;

use anyhow::Result;
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
    eprintln!("Starting stream loop");
    // For data
    let (tx, mut rx) = mpsc::channel(20);
    // For alarm
    let (tx1, mut rx1) = mpsc::channel(1);

    if d != 0 {
        // setup alarm
        //
        eprintln!("setup alarm");
        tokio::spawn(async move {
            thread::sleep(Duration::from_secs(SLEEP));
            tx1.send("bing!").await.unwrap();
        });
    }

    // setup ctrl-c handled
    //
    let mut sig = ctrl_c()?;

    // start working
    //
    eprintln!("working...");
    tokio::spawn(async move {
        loop {
            thread::sleep(Duration::from_secs(2_u64));
            tx.send(".").await.unwrap();
        }
    });

    let mut output = String::new();

    eprintln!("get data thread");
    loop {
        tokio::select! {
            Some(msg) = rx.recv() => output.push_str(msg),
            Some(msg) = rx1.recv() => {
                output.push_str(&format!("{}:{}", msg, "finished!"));
                break;
            },
            _ = sig.recv() => {
                eprintln!("out!");
                break;
            }
        }
        writeln!(out, "{}", output)?;
        out.flush()?;
    }
    writeln!(out, "{}", output)?;
    Ok(out.flush()?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut out = stderr();

    worker_thread(&mut out, SLEEP).await?;

    println!("with sleeper, nothing is displayed");
    std::process::exit(0);
}
