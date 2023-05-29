//! Implementation of some basic `Runnable` tasks.
//!
//! These are here for future enhancements like having a DSL describing a task and this would
//! be some of the "words" the DSL would compile into.
//!

use std::fmt::Debug;
use std::sync::mpsc::Sender;

use anyhow::Result;

use crate::Runnable;

// -----

/// NOP
///
#[derive(Clone, Debug)]
pub struct Nothing {}

impl<T> Runnable<T> for Nothing {
    fn run(&self, tx: Sender<T>) -> Result<()> {
        tx.send("NOP".to_string())
    }
}

// -----

/// Just display a message
///
#[derive(Clone, Debug)]
pub struct Message {
    /// What to display
    msg: String,
}

impl Message {
    #[inline]
    pub fn new(s: &str) -> Self {
        Message { msg: s.to_owned() }
    }
}

impl<T> Runnable<T> for Message {
    fn run(&self, tx: Sender<T>) -> Result<()> {
        tx.send(self.msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nothing_run() {
        let t = Nothing {};

        let r = t.run();
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!("NOP", r);
    }

    #[test]
    fn test_message_run() {
        let m = Message::new("the brown fox");
        let s = m.run();
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!("the brown fox", s);
    }
}
