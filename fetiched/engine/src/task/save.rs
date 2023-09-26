//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::fs;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use eyre::Result;
use tracing::trace;

use engine_macros::RunnableDerive;

use crate::{Runnable, IO};

/// The Save task
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Save {
    /// I/O capabilities
    io: IO,
    /// name for the task
    pub name: String,
    /// File path
    pub path: Option<PathBuf>,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Save {
    /// Initialize our environment
    ///
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        trace!("New Save {}", name);
        Save {
            io: IO::Consumer,
            name: name.to_owned(),
            path: None,
            args: "".to_string(),
        }
    }

    /// Set the input path (for files)
    ///
    pub fn path(&mut self, name: &str) -> &mut Self {
        trace!("Add path: {}", name);
        self.path = Some(PathBuf::from(name));
        self
    }

    /// The heart of the matter: save data
    ///
    #[tracing::instrument]
    pub fn execute(&mut self, data: String, _stdout: Sender<String>) -> Result<()> {
        trace!("Save::execute()");

        if self.path.is_none() {
            trace!("...into stdout");

            Ok(write!(stdout(), "{}", data)?)
        } else {
            let p = self.path.clone().unwrap();
            trace!("... into {}", p.to_string_lossy());

            Ok(fs::write(p, &data)?)
        }
    }
}

impl Default for Save {
    fn default() -> Self {
        Save::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_new() {
        let t = Save::new("foo");

        assert_eq!("foo", t.name);
        assert!(t.path.is_none());
    }

    #[test]
    fn test_write_stdout() {
        let mut t = Save::new("foo");
        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        assert_eq!(PathBuf::from("/nonexistent"), t.path.unwrap());
    }

    #[test]
    fn test_write_file() {
        let mut t = Save::new("foo");
        t.path("../Cargo.toml");

        assert_eq!("foo", t.name);
        assert_eq!(PathBuf::from("../Cargo.toml"), t.path.unwrap());
    }
}
