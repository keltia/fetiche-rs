use enum_dispatch::enum_dispatch;
use std::fmt::Debug;

use eyre::Result;
use rand::Rng;

/// This trait define an object that can be calculated
///
#[enum_dispatch(Task)]
pub trait Calculate: Debug {
    fn execute(&self) -> Stat;
}

// -----

#[derive(Debug)]
pub enum Stat {
    One(usize),
    Two(f64),
}

// -----

#[derive(Debug)]
pub struct Batch {
    inner: Vec<Task>,
}

impl Batch {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn add(&mut self, task: Task) -> &mut Self {
        let _ = &self.inner.push(task);
        self
    }

    pub fn run(&mut self) -> Vec<Stat> {
        let res: Vec<Stat> = self.inner.iter().map(|e| e.execute()).collect();
        eprintln!("res={:?}", res);
        res
    }
}

#[enum_dispatch]
#[derive(Clone, Debug)]
pub enum Task {
    Foo,
    Bar,
}

#[derive(Clone, Debug)]
pub struct Foo {
    pub m: usize,
}

impl Foo {
    pub fn new(m: usize) -> Self {
        Self { m }
    }
}

impl Calculate for Foo {
    fn execute(&self) -> Stat {
        let mut rng = rand::thread_rng();
        let res: usize = rng.gen();
        let res = res / self.m;
        Stat::One(res)
    }
}

#[derive(Clone, Debug)]
pub struct Bar {
    pub f: f64,
}

impl Bar {
    pub fn new(f: f64) -> Self {
        Self { f }
    }
}

impl Calculate for Bar {
    fn execute(&self) -> Stat {
        let mut rng = rand::thread_rng();
        let res: f64 = rng.gen();
        let res = res / self.f;
        Stat::Two(res)
    }
}

fn main() -> Result<()> {
    let t1 = Foo::new(2);
    let t2 = Bar::new(4.0);

    let c1 = Task::from(t1);
    let c2 = Task::from(t2);

    let r1 = c1.execute();
    let r2 = c2.execute();

    dbg!(r1, r2);

    let mut b = Batch::new();
    b.add(c1.clone()).add(c2.clone());

    let res: Vec<Stat> = b.run();
    dbg!(&res);
    Ok(())
}
