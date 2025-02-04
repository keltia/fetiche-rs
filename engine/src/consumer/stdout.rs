use fetiche_macros::RunnableDerive;
use std::sync::mpsc::Sender;

use crate::IO;

#[derive(Clone, Debug, PartialEq, RunnableDerive)]
pub struct Stdout {
    io: IO,
}

impl Stdout {
    #[tracing::instrument]
    pub fn new() -> Self {
        Self { io: IO::Consumer }
    }

    #[tracing::instrument]
    pub fn execute(&self, data: String, _out: Sender<String>, _args: String) {
        println!("{}", data);
    }
}

impl Stdout {
    #[tracing::instrument]
    pub fn new() -> Self {
        Self::new()
    }
}


