//! Implementation of some basic `Runnable` tasks.
//!
//! These are here for future enhancements like having a DSL describing a task and this would
//! be some of the "words" the DSL would compile into.
//!

use std::fmt::Debug;

use eyre::Result;
use tokio::sync::mpsc::Sender;

use fetiche_macros::RunnableDerive;

use crate::{IO, Middle, Runnable, Tee};

// -----

/// NOP
///
#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Nothing {
    io: IO,
}

impl From<Nothing> for Middle {
    fn from(t: Nothing) -> Self {
        Middle::Nothing(t.clone())
    }
}

impl Nothing {
    #[inline]
    #[tracing::instrument]
    pub fn new() -> Self {
        Nothing { io: IO::Producer }
    }

    #[inline]
    #[tracing::instrument]
    pub async fn execute(&self, data: String, stdout: Sender<String>) -> Result<()> {
        Ok(stdout.send(format!("{}|NOP", data)).await?)
    }
}

impl Default for Nothing {
    fn default() -> Self {
        Self::new()
    }
}

/// Copy
///
#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Copy {
    /// I/O capabilities
    io: IO,
}

impl From<Copy> for Middle {
    fn from(t: Copy) -> Self {
        Middle::Copy(t.clone())
    }
}

impl Copy {
    #[inline]
    #[tracing::instrument]
    pub fn new() -> Self {
        Copy { io: IO::Filter }
    }

    #[inline]
    #[tracing::instrument]
    pub async fn execute(&self, data: String, stdout: Sender<String>) -> Result<()> {
        Ok(stdout.send(data).await?)
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
#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Message {
    /// I/O capabilities
    io: IO,
    /// What to display
    msg: String,
}

impl From<Message> for Middle {
    fn from(t: Message) -> Self {
        Middle::Message(t.clone())
    }
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
    pub async fn execute(&self, _data: String, stdout: Sender<String>) -> Result<()> {
        Ok(stdout.send(self.msg.to_string()).await?)
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            io: IO::Filter,
            msg: "".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

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

    #[tokio::test]
    async fn test_nothing_execute() {
        let nothing = Nothing::new();
        let (tx, mut rx) = mpsc::channel(1);

        let input_data = "TestData".to_string();
        nothing.execute(input_data.clone(), tx).unwrap();

        let result = rx.recv().await.unwrap();
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

    #[tokio::test]
    async fn test_copy_execute() {
        let copy = Copy::new();
        let (tx, mut rx) = mpsc::channel(1);

        let input_data = "TestCopyData".to_string();
        copy.execute(input_data.clone(), tx).unwrap();

        let result = rx.recv().await.unwrap();
        assert_eq!(result, input_data);
    }

    #[test]
    fn test_message_new() {
        let msg = "Hello, world!";
        let message = Message::new(msg);

        assert_eq!(message.msg, msg);
        assert_eq!(matches!(message.io, IO::Producer), true);
    }

    #[tokio::test]
    async fn test_message_execute() {
        let msg = "TestMessageContent".to_string();
        let message = Message::new(&msg);
        let (tx, mut rx) = mpsc::channel(1);

        let input_data = "UnusedData".to_string();
        message.execute(input_data, tx).unwrap();

        let result = rx.recv().await.unwrap();
        assert_eq!(result, msg);
    }
}
