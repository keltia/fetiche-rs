use crate::{Consumer, Runnable, Task, IO};
pub use common::*;
pub use convert::*;
use std::fmt::Display;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use strum::EnumString;
pub use tee::*;
use tracing::error;

mod common;
mod convert;
mod tee;

/// Represents different types of filters that can be applied to the data
/// in the processing pipeline.
///
/// Each variant corresponds to a specific filtering strategy that processes
/// or transforms the data as it flows through the pipeline. Filters can modify,
/// duplicate, or pass through data without modification depending on their type.
///
#[derive(Clone, Debug, Default, EnumString, PartialEq, strum::VariantNames)]
pub enum Middle {
    /// Filter that transforms data from one format to another
    Convert(Convert),
    /// Filter that creates an identical copy of the incoming data
    Copy(Copy),
    /// Filter that processes or transforms messages in the data stream
    Message(Message),
    /// Filter that passes data through without any modification
    Nothing(Nothing),
    /// Filter that creates a copy of the data stream while passing through
    Tee(Tee),
    /// Default value.
    #[default]
    Invalid,
}

impl From<Middle> for Task {
    fn from(value: Middle) -> Self {
        Task::Middle(value)
    }
}

impl Runnable for Middle {
    fn cap(&self) -> IO {
        IO::Consumer
    }

    fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<eyre::Result<()>>) {
        match self {
            Middle::Convert(c) => c.run(out),
            Middle::Copy(c) => c.run(out),
            Middle::Message(m) => m.run(out),
            Middle::Nothing(n) => n.run(out),
            Middle::Tee(t) => t.run(out),
            Middle::Invalid => {
                error!("Invalid middleware: {}", self);
                panic!("Invalid middleware: {}", self);
            }
        }
    }
}

impl Display for Middle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Middle::Convert(_) => write!(f, "Convert"),
            Middle::Copy(_) => write!(f, "Copy"),
            Middle::Message(_) => write!(f, "Message"),
            Middle::Nothing(_) => write!(f, "Nothing"),
            Middle::Tee(_) => write!(f, "Tee"),
            Middle::Invalid => write!(f, "Invalid"),
        }
    }
}

