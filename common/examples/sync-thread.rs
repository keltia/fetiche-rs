// Normal sync-based worker/alarm threads
//

use std::io::{stderr, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use eyre::Result;
use fetiche_common::{close_logging, init_logging};
use tracing::trace;

// If 0, infinite wait, need SIGINT to sop
//
const SLEEP: u64 = 20;

fn worker_thread(out: &mut dyn Write, d: u64) -> Result<()> {

    // Launch it!
    //
    eprintln!("Starting stream loop");
    // For data & alarm
    let (tx, rx) = mpsc::channel();

    if d != 0 {
        // setup alarm
        //
        let tx1 = tx.clone();
        eprintln!("setup alarm");
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(SLEEP));
            tx1.send("bing!").unwrap();
        });
    }

    // start working
    //
    eprint!("working...");
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(2_u64));
        tx.send(".").unwrap();
    });

    let mut output = String::new();

    eprintln!("get data thread");
    loop {
        match rx.recv() {
            Ok(msg) => match msg {
                "bing!" => {
                    eprintln!("alarm, out!");
                    break;
                }
                _ => output.push_str(msg),
            },
            _ => continue,
        }
        writeln!(out, "{}", output)?;
        out.flush()?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut out = stderr();

    let _ = init_logging("sync-thread", false, true, None);

    let _ = ctrlc::set_handler(move || {
        trace!("Ctrl-C pressed");
        close_logging();
        std::process::exit(0);
    });

    worker_thread(&mut out, SLEEP)?;

    println!("with sleeper, nothing is displayed");
    close_logging();
    std::process::exit(0);
}
