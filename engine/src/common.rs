//! Implementation of some basic `Runnable` tasks.
//!
//! These are here for future enhancements like having a DSL describing a task and this would
//! be some of the "words" the DSL would compile into.
//!

use std::fmt::Debug;
use std::io::Write;

use anyhow::Result;

use crate::Runnable;

// -----

/// NOP
///
#[derive(Clone, Debug)]
pub struct Nothing {}

impl Runnable for Nothing {
    fn run(&mut self, out: &mut dyn Write) -> Result<()> {
        Ok(write!(out, "NOP")?)
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

impl Runnable for Message {
    fn run(&mut self, out: &mut dyn Write) -> Result<()> {
        Ok(write!(out, "{}", self.msg)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nothing_run() {
        let mut t = Nothing {};

        let mut data = vec![];
        let r = t.run(&mut data);

        let r = String::from_utf8(data);
        assert!(r.is_ok());
        let r = r.unwrap();
        assert_eq!("NOP", r);
    }

    #[test]
    fn test_message_run() {
        let mut m = Message::new("the brown fox");

        let mut data = vec![];

        let s = m.run(&mut data);
        let s = String::from_utf8(data);

        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!("the brown fox", s);
    }
}
