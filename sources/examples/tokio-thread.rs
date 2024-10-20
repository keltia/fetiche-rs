// Tokio-based worker/alarm threads
//
// We may be going full async, hang on Baby, we're for a ride!
//

use std::io::{stderr, Write};
use std::time::Duration;

use eyre::Result;
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;
use tokio::sync::mpsc;
use tokio::time::sleep;

// If 0, infinite wait, need SIGINT to sop
//
const SLEEP: u64 = 20;
const WAIT: Duration = Duration::from_secs(2u64);

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
            sleep(Duration::from_secs(SLEEP)).await;
            tx1.send("\nbing!").await.unwrap();
        });
    }

    // setup ctrl-c handled
    //
    #[cfg(windows)]
    let mut sig = ctrl_c()?;

    #[cfg(unix)]
    let mut stream = signal(SignalKind::interrupt())?;

    // start working
    //
    eprintln!("working...");
    tokio::spawn(async move {
        loop {
            sleep(WAIT).await;
            if let Err(_) = tx.send(".").await {
                break;
            }
        }
    });

    let mut output = String::new();

    eprintln!("get data thread");
    loop {
        #[cfg(windows)]
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
        #[cfg(unix)]
        tokio::select! {
            Some(msg) = rx.recv() => output.push_str(msg),
            Some(msg) = rx1.recv() => {
                output.push_str(&format!("{}:{}", msg, "finished!"));
                break;
            },
            Some(_) = stream.recv() => {
                eprintln!("Got SIGINT");
                break;
            },
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

    eprintln!("with sleeper, nothing is displayed");
    Ok(())
}
