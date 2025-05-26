//! `Read` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::path::PathBuf;

use chrono::Utc;
use eyre::Result;
use std::sync::mpsc::Sender;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::trace;

use fetiche_formats::Format;
use fetiche_macros::RunnableDerive;

use crate::{EngineStatus, Producer, Runnable, Stats, IO};

/// The Read task
///
#[derive(Clone, Debug, PartialEq, RunnableDerive)]
pub struct Read {
    /// I/O capabilities
    io: IO,
    /// name for the task
    pub name: String,
    /// If we need a specific format as output.
    pub format: Format,
    /// File path
    pub path: Option<PathBuf>,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl From<Read> for Producer {
    fn from(f: Read) -> Self {
        Producer::Read(f)
    }
}

impl Read {
    /// Initialize our environment
    ///
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        Read {
            io: IO::Producer,
            name: name.to_owned(),
            format: Format::None,
            path: None,
            args: "".to_string(),
        }
    }

    /// Set the input path (for files)
    ///
    #[tracing::instrument(skip(self))]
    pub fn path(&mut self, name: &str) -> &mut Self {
        self.path = Some(PathBuf::from(name));
        self
    }

    #[tracing::instrument(skip(self))]
    pub fn fmt(&mut self, f: Format) -> &mut Self {
        self.format = f;
        self
    }

    /// The heart of the matter: fetch data
    ///
    #[tracing::instrument(skip(self))]
    pub async fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<Stats> {
        trace!("Read::transform()");
        if self.path.is_none() {
            Err(EngineStatus::UninitialisedRead.into())
        } else {
            let p = self.path.clone().unwrap();
            let mut lines = String::new();
            let size = File::open(&p).await?.read_to_string(&mut lines).await?;

            // Now send each line down the pipe (while counting)
            //
            let _ = stdout.send(lines)?;
            Ok(Stats {
                tm: Utc::now().timestamp() as u64,
                bytes: size as u64,
                pkts: 1u32,
                ..Default::default()
            })
        }
    }
}

impl Default for Read {
    fn default() -> Self {
        Read::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_read_new() {
        let t = Read::new("foo");

        assert_eq!("foo", t.name);
        assert!(t.path.is_none());
    }

    #[test]
    fn test_read_none() {
        let mut t = Read::new("foo");
        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        assert_eq!(PathBuf::from("/nonexistent"), t.path.unwrap());
    }

    #[test]
    fn test_read_file() {
        let mut t = Read::new("foo");
        t.path("../Cargo.toml");
        t.fmt(Format::Asd);

        assert_eq!("foo", t.name);
        assert_eq!(PathBuf::from("../Cargo.toml"), t.path.unwrap());
    }

    #[tokio::test]
    async fn test_read_execute_uninitialized() {
        let mut t = Read::new("foo");
        let (tx, rx) = channel();

        // Execute should fail as path and format are not set
        let result = t.execute(String::new(), tx).await;
        assert!(result.is_err());
        match result {
            Err(e) => assert_eq!(
                format!("{}", e),
                format!("{}", EngineStatus::UninitialisedRead)
            ),
            _ => panic!("Expected an error"),
        }
    }

    #[tokio::test]
    async fn test_read_execute_with_nonexistent_file() {
        let mut t = Read::new("foo");
        t.path("/nonexistent");
        let (tx, rx) = channel();

        // Execute should fail as file does not exist
        let result = t.execute(String::new(), tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_execute_with_valid_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary file and write some data to it
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line1").unwrap();
        writeln!(temp_file, "line2").unwrap();
        write!(temp_file, "line3").unwrap();

        let mut t = Read::new("foo");
        t.path(temp_file.path().to_str().unwrap());
        let (tx, rx) = channel();

        // Execute should succeed and send lines to channel
        let result = t.execute(String::new(), tx).await;
        assert!(result.is_ok());

        let mut lines: Vec<String> = vec![];
        for received in rx {
            lines.push(received);
        }
        let res = vec![["line1", "line2", "line3"].join("\n")];
        assert_eq!(lines, res);
    }
}
