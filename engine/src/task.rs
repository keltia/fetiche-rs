//! Implementation of some basic `Runnable` tasks.
//!

use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::{debug, trace};

use fetiche_formats::Format;
use fetiche_sources::{Fetchable, Filter};

use crate::{Input, Runnable};

// -----

/// NOP
///
#[derive(Clone, Debug)]
pub struct Nothing {}

impl Nothing {
    pub fn new() -> Self {
        Self {}
    }
}

impl Runnable for Nothing {
    fn run(&self) -> Result<String> {
        Ok("NOP".to_string())
    }
}

// -----

/// Just display a message
///
#[derive(Clone, Debug)]
pub struct Message {
    pub msg: String,
}

impl Message {
    pub fn new(s: &str) -> Self {
        Message { msg: s.to_owned() }
    }
}

impl Runnable for Message {
    fn run(&self) -> Result<String> {
        Ok(self.msg.to_string())
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
}
