//! Implementation of some basic `Runnable` tasks.
//!
//! These are here for future enhancements like having a DSL describing a task and this would
//! be some of the "words" the DSL would compile into.
//!

use std::fmt::Debug;
use std::sync::mpsc::Sender;

use eyre::Result;

use fetiche_macros::RunnableDerive;

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
        Nothing { io: IO::Producer }
    }

    #[inline]
    #[tracing::instrument]
    fn execute(&self, data: String, stdout: Sender<String>) -> Result<()> {
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
/// FIXME: went from a `Filter` to a `Producer` to satisfy `Job` requirements.
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
            io: IO::Producer,
            msg: s.to_owned(),
        }
    }

    #[inline]
    #[tracing::instrument]
    fn execute(&self, _data: String, stdout: Sender<String>) -> Result<()> {
        Ok(stdout.send(self.msg.to_string())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_nothing_new() {
        let nothing = Nothing::new();
        assert_eq!(matches!(nothing.io, IO::Producer), true);
    }

    #[test]
    fn test_nothing_default() {
        let nothing = Nothing::default();
        assert_eq!(matches!(nothing.io, IO::Producer), true);
    }

    #[test]
    fn test_nothing_execute() {
        let nothing = Nothing::new();
        let (tx, rx) = mpsc::channel();

        let input_data = "TestData".to_string();
        nothing.execute(input_data.clone(), tx).unwrap();

        let result = rx.recv().unwrap();
        assert_eq!(result, format!("{}|NOP", input_data));
    }

    #[test]
    fn test_copy_new() {
        let copy = Copy::new();
        assert_eq!(matches!(copy.io, IO::Filter), true);
    }

    #[test]
    fn test_copy_default() {
        let copy = Copy::default();
        assert_eq!(matches!(copy.io, IO::Filter), true);
    }

    #[test]
    fn test_copy_execute() {
        let copy = Copy::new();
        let (tx, rx) = mpsc::channel();

        let input_data = "TestCopyData".to_string();
        copy.execute(input_data.clone(), tx).unwrap();

        let result = rx.recv().unwrap();
        assert_eq!(result, input_data);
    }

    #[test]
    fn test_message_new() {
        let msg = "Hello, world!";
        let message = Message::new(msg);

        assert_eq!(message.msg, msg);
        assert_eq!(matches!(message.io, IO::Producer), true);
    }

    #[test]
    fn test_message_execute() {
        let msg = "TestMessageContent".to_string();
        let message = Message::new(&msg);
        let (tx, rx) = mpsc::channel();

        let input_data = "UnusedData".to_string();
        message.execute(input_data, tx).unwrap();

        let result = rx.recv().unwrap();
        assert_eq!(result, msg);
    }
}
