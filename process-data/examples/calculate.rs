use std::fmt::Debug;
use enum_dispatch::enum_dispatch;

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
    inner: Vec<Box<dyn Calculate>>,
}

impl Batch {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }

    pub fn add(&mut self, task: Box<dyn Calculate>) -> &mut Self {
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
#[derive(Debug)]
pub enum Task {
    Foo(usize),
    Bar(f64),
}

#[derive(Debug)]
struct Foo {
    m: usize,
}

impl Foo {
    pub fn new() -> Self {
        Self { m: 0 }
    }
}

impl Calculate for Foo {
    fn execute(&self) -> Stat {
        let mut rng = rand::thread_rng();
        let res: usize = rng.gen();
        Stat::One(res)
    }
}

#[derive(Debug)]
struct Bar {
    pub f: f64,
}

impl Bar {
    pub fn new() -> Self {
        Self { f: 0. }
    }
}

impl Calculate for Bar {
    fn execute(&self) -> Stat {
        let mut rng = rand::thread_rng();
        let res: f64 = rng.gen();
        Stat::Two(res)
    }
}


fn main() -> Result<()> {
    let c1 = Task::Foo::new();
    let c2 = Bar::new();

    let r1 = c1.execute();
    let r2 = c2.execute();

    dbg!(r1, r2);

    let b = Batch::new().add(Box::new(c1)).add(Box::new(c2));

    let res: Vec<Stat> = b.run();
    dbg!(&res);
    Ok(())
}
