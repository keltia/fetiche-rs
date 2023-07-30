//! Implementation of some basic `Runnable` tasks.
//!
//! These are here for future enhancements like having a DSL describing a task and this would
//! be some of the "words" the DSL would compile into.
//!

use std::fmt::Debug;
use std::sync::mpsc::Sender;

use eyre::Result;

use engine_macros::RunnableDerive;

use crate::{Runnable, IO};

// -----

/// NOP
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Nothing {
    io: IO,
}

impl Nothing {
    #[inline]
    #[tracing::instrument]
    pub fn new() -> Self {
        Nothing { io: IO::Filter }
    }

    #[inline]
    #[tracing::instrument]
    fn execute(&self, data: String, stdout: Sender<String>) -> Result<()> {
        dbg!(&data);
        Ok(stdout.send(format!("{}|NOP", data))?)
    }
}

impl Default for Nothing {
    fn default() -> Self {
        Self::new()
    }
}

/// Copy
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Copy {
    /// I/O capabilities
    io: IO,
}

impl Copy {
    #[inline]
    #[tracing::instrument]
    pub fn new() -> Self {
        Copy { io: IO::Filter }
    }

    #[inline]
    #[tracing::instrument]
    fn execute(&self, data: String, stdout: Sender<String>) -> Result<()> {
        dbg!(&data);
        Ok(stdout.send(data)?)
    }
}

impl Default for Copy {
    fn default() -> Self {
        Self::new()
    }
}

/// Just display a message
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Message {
    /// I/O capabilities
    io: IO,
    /// What to display
    msg: String,
}

impl Message {
    #[inline]
    #[tracing::instrument]
    pub fn new(s: &str) -> Self {
        Message {
            io: IO::Filter,
            msg: s.to_owned(),
        }
    }

    #[inline]
    #[tracing::instrument]
    fn execute(&self, data: String, stdout: Sender<String>) -> Result<()> {
        Ok(stdout.send(format!("{}|{}", data, self.msg))?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;

    #[test]
    fn test_nothing_run() {
        let mut t = Nothing::new();

        let (tx, rx) = channel();

        let (r, h) = t.run(rx);

        let r = r.recv();
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!("|NOP", r);
    }

    #[test]
    fn test_message_run() {
        let mut m = Message::new("the brown fox");

        let (tx, rx) = channel();

        let (r, h) = m.run(rx);

        let r = r.recv();
        assert!(r.is_ok());
        let s = r.unwrap();
        assert_eq!("|the brown fox", s);
    }
}
