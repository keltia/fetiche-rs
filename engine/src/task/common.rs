//! Implementation of some basic `Runnable` tasks.
//!
//! These are here for future enhancements like having a DSL describing a task and this would
//! be some of the "words" the DSL would compile into.
//!

use std::fmt::Debug;
use std::sync::mpsc::Sender;

use anyhow::Result;

use engine_macros::RunnableDerive;

use crate::Runnable;

// -----

/// NOP
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Nothing {}

impl Nothing {
    #[inline]
    fn execute(&self, data: String, stdout: Sender<String>) -> Result<()> {
        Ok(stdout.send(format!("{}|NOP", data))?)
    }
}

/// Just display a message
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Message {
    /// What to display
    msg: String,
}

impl Message {
    #[inline]
    pub fn new(s: &str) -> Self {
        Message { msg: s.to_owned() }
    }

    #[inline]
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
        let mut t = Nothing {};

        let (tx, rx) = channel();

        let mut data = vec![];
        let (r, h) = t.run(rx);

        let r = r.recv();
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!("NOP", r);
    }

    #[test]
    fn test_message_run() {
        let mut m = Message::new("the brown fox");

        let (tx, rx) = channel();

        let mut data = vec![];
        let (r, h) = m.run(rx);

        let r = r.recv();
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!("the brown fox", s);
    }
}
