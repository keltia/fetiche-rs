use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use eyre::Result;
use rand::Rng;
use std::fmt::Debug;

/// This trait define an object that can be calculated
///
#[async_trait]
#[enum_dispatch(Task)]
pub trait Calculate: Debug {
    async fn execute(&self) -> Stat;
}

// -----

#[derive(Debug)]
pub enum Stat {
    One(u32),
    Two(f64),
}

// -----

#[enum_dispatch]
#[derive(Clone, Debug)]
pub enum Task {
    Foo,
    Bar,
}

// -----

#[derive(Clone, Debug)]
pub struct Foo {
    pub m: u32,
}

impl Foo {
    pub fn new(m: u32) -> Self {
        Self { m }
    }
}

#[async_trait]
impl Calculate for Foo {
    async fn execute(&self) -> Stat {
        let mut rng = rand::rng();
        let res: u32 = rng.random();
        let res = res / self.m;
        Stat::One(res)
    }
}

// -----

#[derive(Clone, Debug)]
pub struct Bar {
    pub f: f64,
}

impl Bar {
    pub fn new(f: f64) -> Self {
        Self { f }
    }
}

#[async_trait]
impl Calculate for Bar {
    async fn execute(&self) -> Stat {
        let mut rng = rand::rng();
        let res: f64 = rng.random();
        let res = res / self.f;
        Stat::Two(res)
    }
}

// -----

#[tokio::main]
async fn main() -> Result<()> {
    let t1 = Foo::new(2);
    let t2 = Bar::new(4.0);

    let c1 = Task::from(t1);
    let c2 = Task::from(t2);

    let r1 = c1.execute().await;
    let r2 = c2.execute().await;

    dbg!(r1, r2);

    Ok(())
}
