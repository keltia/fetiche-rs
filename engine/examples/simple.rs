//! Basic framework for the runner
//!
//! cf. [Playground](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2021&gist=7fcf8265fc664d887e97959c61a18f6c)
//!
//! this is the bare version without using the macro.
//!

use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::{stdout, Write};
use std::sync::mpsc::*;
use std::thread;
use std::thread::*;

pub trait Runnable: Debug {
    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>);
}

#[derive(Debug)]
struct Counter {
    cnt: usize,
}

impl Runnable for Counter {
    fn run(&mut self, rx: Receiver<String>) -> (Receiver<String>, JoinHandle<Result<()>>) {
        let (tx1, rx1) = channel::<String>();

        let cnt = self.cnt.clone();
        let h = thread::spawn(move || {
            eprintln!("counter");
            for data in rx {
                // send our data
                for i in cnt..(cnt + 3) {
                    let data = format!("->{},", i);
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
        for p in pids {
            p.join().unwrap();
        }
    }
}

fn main() {
    let t1 = Counter { cnt: 1 };
    let t2 = Msg {
        msg: "bnar".to_string(),
    };
    let t3 = Counter { cnt: 42 };

    let mut j = Job::new("test");

    j.add(Box::new(t1)).add(Box::new(t2)).add(Box::new(t3));

    dbg!(&j);

    j.run(&mut stdout());
}
