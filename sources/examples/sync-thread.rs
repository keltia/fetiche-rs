// Normal sync-based worker/alarm threads
//

use std::env::Args;
use std::io::{stderr, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use std::{env, thread, time};

use anyhow::Result;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;

// If 0, infinite wait, need SIGINT to sop
//
const SLEEP: u64 = 20;

fn worker_thread(out: &mut dyn Write, d: u64) -> Result<()> {
    let term = Arc::new(AtomicBool::new(false));

    // Setup signals
    //
    // NOTE: SIGINT must be issued twice to immediately stop, not sure is it needed.
    //
    // NOTE: on Windows, single register does not work, picture me surprised (not)
    //
    for sig in TERM_SIGNALS {
        flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term))?;
        flag::register(*sig, Arc::clone(&term))?;
    }

    // Launch it!
    //
    while !term.load(Ordering::Relaxed) {
        // Launch it!
        //
        writeln!(stderr(), "Starting stream loop")?;
        // For data & alarm
        let (tx, mut rx) = mpsc::channel();

        if d != 0 {
            // setup alarm
            //
            let tx1 = tx.clone();
            writeln!(stderr(), "setup alarm")?;
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(SLEEP));
                tx1.send("bing!");
                return;
            });
        }

        // start working
        //
        writeln!(stderr(), "working...");
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(2_u64));
            tx.send(".");
        });

        let mut output = String::new();

        writeln!(stderr(), "get data thread")?;
        loop {
            match rx.recv() {
                Ok(msg) => match msg {
                    "bing!" => {
                        writeln!(stderr(), "alarm, out!")?;
                        break;
                    }
                    _ => output.push_str(msg),
                },
                _ => continue,
            }
            writeln!(out, "{}", output)?;
            out.flush();
        }
        return Ok(());
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut out = stderr();

    let args = env::args();

    worker_thread(&mut out, SLEEP)?;

    println!("with sleeper, nothing is displayed");
    std::process::exit(0);
}
