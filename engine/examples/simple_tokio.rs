//! A Tokio runner with one task per pipeline stage.
//!
//! Run it with `cargo run -p fetiche-engine --example simple_tokio`. The
//! example builds a job from two counters and a message stage and sends
//! a single input through them.
//!
//! ## Architecture
//!
//! `#[tokio::main]` starts the Tokio runtime. `Job` stores stages in a
//! `Vec<Box<dyn Runnable>>` in insertion order. Each stage takes ownership
//! of an input receiver, creates an output channel, and starts a task with
//! `tokio::spawn`. The stage returns its output receiver and task handle.
//! `Job::run` passes each receiver to the next stage, retains the task
//! handles, and drains the final receiver into an `AsyncWrite` target.
//!
//! ```text
//! Job input -> Counter(tag=1, cnt=1) -> Msg("bnar") -> Counter(tag=3, cnt=42) -> output
//! ```
//!
//! ## Message flow and shutdown
//!
//! 1. The job sends one empty string and drops the input sender. Each
//!    stage awaits messages until its upstream sender is gone and the
//!    buffered messages have been consumed.
//! 2. The first counter prints its input and sends `t1 ->1,`,
//!    `t1 ->2,`, and `t1 ->3,`. When its input closes, it sends `end`
//!    and exits, dropping its output sender.
//! 3. `Msg` forwards those four strings in order. When its input closes,
//!    it sends `bnar` and exits.
//! 4. The final counter prints each of its five inputs and emits
//!    `t3 ->42,`, `t3 ->43,`, and `t3 ->44,` for each one. The
//!    first counter's `end` is a regular message. Channel closure causes
//!    the final counter to send its own `end` and exit.
//! 5. The job writes and flushes one `received: (...)` line per message.
//!    The example uses Tokio stdout and produces 16 such lines. After it
//!    drains the final channel, the job awaits every task handle.
//!
//! ## Process lifecycle
//!
//! `main` initializes logging and reads `/tmp/simple.pid` through
//! `tokio::fs`. A missing file allows the job to run. An existing file
//! causes `main` to return an error containing the PID if parsing succeeds.
//! Other read and parse errors also propagate. The example runs in the
//! foreground and creates no PID file. After a successful job and stdout
//! flush, it awaits a five-second Tokio timer and attempts to remove the
//! PID-file path, ignoring any removal error.
//!
//! ## Differences from `simple.rs`
//!
//! - Tokio schedules the stage tasks on its runtime. Each channel buffers up
//!   to 16 messages, and a send waits for capacity when the buffer is full.
//!   In `simple.rs`, each stage owns a dedicated OS thread and the channels
//!   are unbounded. Both examples emit the same ordered `received:` lines.
//! - Pipeline writes and flushes, the PID-file operations, and the delay
//!   are awaited. The stage diagnostics still use synchronous `println!`
//!   and `eprintln!`, and their interleaving with output can vary.
//! - On a pipeline error, the job drops the final receiver so pending sends
//!   can fail and stages can exit as their receivers are dropped. It awaits every
//!   task and returns the first error it observes, checking the pipeline
//!   result first and then the task handles in stage order. Task panics
//!   surface as join errors. `simple.rs` drops the worker handles and
//!   uses `unwrap` for pipeline writes and flushes.
//! - A valid PID file causes an error return from async `main`. The
//!   threaded version prints the PID and explicitly exits with status 1.

use std::fmt::Debug;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use eyre::{Result, eyre};
use fetiche_common::init_logging;
use tokio::fs;
use tokio::io::{self, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc::{self, Receiver};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::info;

const CHANNEL_CAPACITY: usize = 16;

trait Runnable: Debug {
    fn run(&self, input: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}

#[derive(Copy, Clone, Debug)]
struct Counter {
    tag: usize,
    cnt: usize,
}

impl Runnable for Counter {
    fn run(&self, mut input: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>) {
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let Self { tag, cnt } = *self;
        let task = tokio::spawn(async move {
            eprintln!("counter");
            while let Some(data) = input.recv().await {
                println!("data: {data}");
                for i in cnt..(cnt + 3) {
                    tx.send(format!("t{tag} ->{i},")).await?;
                }
            }
            tx.send("end".to_string()).await?;
            Ok(())
        });
        (rx, task)
    }
}

#[derive(Debug)]
struct Msg {
    msg: String,
}

impl Runnable for Msg {
    fn run(&self, mut input: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>) {
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let msg = self.msg.clone();
        let task = tokio::spawn(async move {
            eprintln!("msg");
            while let Some(data) = input.recv().await {
                tx.send(data).await?;
            }
            tx.send(msg).await?;
            Ok(())
        });
        (rx, task)
    }
}

#[derive(Debug)]
struct Job {
    name: String,
    stages: Vec<Box<dyn Runnable>>,
}

impl Job {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            stages: Vec::new(),
        }
    }

    fn add(&mut self, stage: Box<dyn Runnable>) -> &mut Self {
        self.stages.push(stage);
        self
    }

    async fn run(&self, out: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        eprintln!("starting {}", self.name);
        let (tx, mut output) = mpsc::channel(CHANNEL_CAPACITY);
        let mut tasks = Vec::with_capacity(self.stages.len());

        for stage in &self.stages {
            let (next_output, task) = stage.run(output);
            output = next_output;
            tasks.push(task);
        }

        let mut result: Result<()> = async {
            let sent = tx.send(String::new()).await;
            drop(tx);
            sent?;
            while let Some(msg) = output.recv().await {
                out.write_all(format!("received: ({msg})\n").as_bytes())
                    .await?;
                out.flush().await?;
            }
            Ok(())
        }
        .await;

        // Closing the receiver also releases blocked senders if writing failed.
        drop(output);
        for task in tasks {
            let task_result = task.await.map_err(Into::into).and_then(|result| result);
            result = result.and(task_result);
        }
        result
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("simple_tokio", false, true, None)?;

    let pid = Path::new("/tmp/simple.pid");
    match fs::read_to_string(pid).await {
        Ok(contents) => {
            info!("PID exists");
            let pid = contents.trim_end().parse::<u32>()?;
            return Err(eyre!("Check PID {pid}"));
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }

    let mut stdout = io::stdout();
    let mut job = Job::new("test");
    job.add(Box::new(Counter { tag: 1, cnt: 1 }))
        .add(Box::new(Msg {
            msg: "bnar".to_string(),
        }))
        .add(Box::new(Counter { tag: 3, cnt: 42 }));

    dbg!(&job);
    job.run(&mut stdout).await?;
    stdout.flush().await?;

    info!("sleep");
    sleep(Duration::from_secs(5)).await;

    let _ = fs::remove_file(pid).await;
    Ok(())
}
