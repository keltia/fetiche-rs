//! A thread-per-stage runner built from standard Rust channels.
//!
//! Run it with `cargo run -p fetiche-engine --example simple`. The example
//! shows how to assemble a job from different kinds of stages and pass
//! messages through them. It implements the runner directly without a macro.
//!
//! ## Architecture
//!
//! `Job` stores the stages in insertion order in a `VecDeque<Box<dyn Runnable>>`.
//! The shared `Runnable` trait lets the same job hold both `Counter` and `Msg`.
//! Each stage takes ownership of its input receiver, creates an unbounded
//! `std::sync::mpsc` channel for its output, and spawns a worker thread.
//! It returns the output receiver and the worker's join handle.
//!
//! `Job::run` creates the input channel and folds over the stages, passing
//! each output receiver to the next stage. It keeps the first sender and
//! drains the last receiver on the calling thread. The join handles are
//! collected during setup and dropped when the job returns, so worker return
//! values and panics are never collected by the job.
//!
//! ```text
//! Job input -> Counter(tag=1, cnt=1) -> Msg("bnar") -> Counter(tag=3, cnt=42) -> output
//! ```
//!
//! ## Message flow and shutdown
//!
//! 1. The job sends one empty string to start the first counter, then drops
//!    the input sender. Each worker's receive loop drains its input and ends
//!    when the upstream sender has been dropped.
//! 2. The first counter prints the input and sends `t1 ->1,`, `t1 ->2,`, and
//!    `t1 ->3,`. Once its input closes, it sends `end` and exits, dropping
//!    its output sender.
//! 3. `Msg` forwards all four strings in order. Once its input closes, it
//!    sends `bnar` and exits, closing the next counter's input.
//! 4. The final counter prints each of its five inputs and emits
//!    `t3 ->42,`, `t3 ->43,`, and `t3 ->44,` for each one. The first
//!    counter's `end` is ordinary data here. Channel closure causes the
//!    final counter to send its own `end` and exit.
//! 5. The job writes and flushes one `received: (...)` line per message to
//!    the supplied `Write` target. The example uses stdout and produces 16
//!    such lines. The job returns when the final channel is drained and closed.
//!
//! ## Process lifecycle
//!
//! `main` initializes logging and checks the Unix-style path
//! `/tmp/simple.pid`. If the file exists, it reads and parses the PID,
//! prints it, and exits with status 1. The example runs in the foreground
//! and never creates a PID file. After the pipeline finishes, `main` flushes
//! stdout, sleeps for five seconds, and attempts to remove that path,
//! ignoring any removal error.
//!
//! See also the original [Playground example](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=7fcf8265fc664d887e97959c61a18f6c).

use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::*;
use std::thread::*;
use std::time::Duration;
use std::{fs, io, thread};

use eyre::Result;
use fetiche_common::init_logging;
use tracing::info;

pub trait Runnable: Debug {
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}

#[derive(Copy, Clone, Debug)]
struct Counter {
    tag: usize,
    cnt: usize,
}

impl Counter {
    fn tag(&self) -> usize {
        self.tag
    }
}

impl Runnable for Counter {
    fn run(&mut self, rx: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>) {
        let (tx1, rx1) = channel::<String>();

        let cnt = self.cnt.clone();
        let tag = self.tag();
        let h = thread::spawn(move || {
            eprintln!("counter");
            for data in rx {
                // send our data
                println!("data: {}", data);
                for i in cnt..(cnt + 3) {
                    let data = format!("t{} ->{},", tag, i);
                    if tx1.send(data).is_err() {
                        eprintln!("err");
                        break;
                    }
                }
            }
            tx1.send("end".to_string()).unwrap();
            Ok(())
        });
        (rx1, h)
    }
}

#[derive(Debug)]
struct Msg {
    msg: String,
}

impl Runnable for Msg {
    fn run(&mut self, rx: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>) {
        let (tx1, rx1) = channel::<String>();

        let msg = self.msg.clone();
        let h = thread::spawn(move || {
            eprintln!("msg");
            for data in rx {
                let data = format!("{}", data);
                if tx1.send(data).is_err() {
                    break;
                }
            }
            tx1.send(msg).unwrap();
            Ok(())
        });
        (rx1, h)
    }
}

#[derive(Debug)]
struct Job {
    name: String,
    list: VecDeque<Box<dyn Runnable>>,
}

impl Job {
    pub fn new(s: &str) -> Self {
        Self {
            name: s.to_string(),
            list: VecDeque::new(),
        }
    }

    pub fn add(&mut self, t: Box<dyn Runnable>) -> &mut Self {
        self.list.push_back(t);
        self
    }

    pub fn run(&mut self, out: &mut dyn Write) {
        eprintln!("starting {}", self.name);
        // setup context tx: stdin / rx: stdout
        let (tx, rx) = channel::<String>();
        let mut pids = vec![];

        let end = self.list.iter_mut().fold(rx, |acc, t| {
            let (rx, h) = t.run(acc);
            pids.push(h);
            rx
        });

        tx.send("".to_string()).unwrap();
        drop(tx);
        for msg in end {
            writeln!(out, "received: ({})", msg).unwrap();
            out.flush().unwrap();
        }
    }
}

fn main() -> Result<()> {
    init_logging("simple", false, true, None)?;

    let pid = PathBuf::from("/tmp/simple.pid");
    if pid.exists() {
        info!("PID exist");
        let pid = fs::read_to_string(&pid)?.trim_end().parse::<u32>()?;
        eprintln!("Check PID {}", pid);
        std::process::exit(1);
    }

    let mut stdout = io::stdout();

    let t1 = Counter { tag: 1, cnt: 1 };
    let t2 = Msg {
        msg: "bnar".to_string(),
    };
    let t3 = Counter { tag: 3, cnt: 42 };

    let mut j = Job::new("test");

    j.add(Box::new(t1)).add(Box::new(t2)).add(Box::new(t3));

    dbg!(&j);

    j.run(&mut stdout);

    let _ = stdout.flush()?;

    info!("sleep");
    sleep(Duration::from_secs(5));

    let _ = fs::remove_file(&pid);

    Ok(())
}
