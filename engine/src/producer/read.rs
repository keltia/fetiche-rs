//! `Read` is a `Runnable` task as defined in the `engine`  crate.
//!

use std::num::NonZeroU32;
use std::path::PathBuf;

use eyre::Result;
use jiff::Timestamp;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::Sender;
use tracing::trace;

use fetiche_formats::Format;
use fetiche_macros::RunnableDerive;

use crate::{EngineStatus, IO, Producer, Runnable, Stats};

pub(crate) const DEFAULT_READ_MAX_BYTES: NonZeroU32 = NonZeroU32::new(64 * 1024 * 1024).unwrap();

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
    max_bytes: NonZeroU32,
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
            max_bytes: DEFAULT_READ_MAX_BYTES,
        }
    }

    /// Set the input path (for files)
    ///
    #[tracing::instrument(skip(self))]
    pub fn path(&mut self, name: &str) -> &mut Self {
        self.path = Some(PathBuf::from(name));
        self
    }

    /// Override the maximum size of the single UTF-8 payload emitted by this stage.
    ///
    /// A file of exactly this size is accepted. A larger file produces
    /// [`EngineStatus::ReadTooLarge`] before any part of it is sent.
    ///
    /// ```
    /// use fetiche_engine::Read;
    /// use std::num::NonZeroU32;
    /// use tokio::sync::mpsc;
    ///
    /// #[tokio::main(flavor = "current_thread")]
    /// async fn main() -> eyre::Result<()> {
    ///     let directory = tempfile::tempdir()?;
    ///     let path = directory.path().join("input.txt");
    ///     tokio::fs::write(&path, "data").await?;
    ///
    ///     let mut reader = Read::new("read input");
    ///     reader.path(path.to_str().unwrap());
    ///     reader.max_bytes(NonZeroU32::new(4).unwrap());
    ///     let (tx, mut rx) = mpsc::channel(1);
    ///
    ///     let stats = reader.execute(String::new(), tx).await?;
    ///     assert_eq!(stats.bytes, 4);
    ///     assert_eq!(rx.recv().await.as_deref(), Some("data"));
    ///     Ok(())
    /// }
    /// ```
    pub fn max_bytes(&mut self, limit: NonZeroU32) -> &mut Self {
        self.max_bytes = limit;
        self
    }

    #[tracing::instrument(skip(self))]
    pub fn fmt(&mut self, f: Format) -> &mut Self {
        self.format = f;
        self
    }

    /// Reads the configured UTF-8 file and sends it downstream as one message.
    ///
    /// Returns file statistics after the send succeeds. A missing path, an
    /// oversized file, invalid UTF-8, or a closed receiver causes an error.
    #[tracing::instrument(skip(self))]
    pub async fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<Stats> {
        trace!("Read::transform()");
        let path = self.path.as_ref().ok_or(EngineStatus::UninitialisedRead)?;
        let max_bytes = u64::from(self.max_bytes.get());
        let mut data = Vec::new();
        let size = File::open(path)
            .await?
            .take(max_bytes + 1)
            .read_to_end(&mut data)
            .await?;
        if size as u64 > max_bytes {
            return Err(EngineStatus::ReadTooLarge {
                path: path.clone(),
                max_bytes,
            }
            .into());
        }

        stdout.send(String::from_utf8(data)?).await?;
        Ok(Stats {
            tm: Timestamp::now().as_second() as u64,
            bytes: size as u64,
            pkts: 1,
            ..Default::default()
        })
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
    use tokio::sync::mpsc::channel;

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
        let (tx, _rx) = channel(1);

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
        let (tx, _rx) = channel(1);

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
        t.max_bytes(NonZeroU32::new(17).unwrap());
        let (tx, mut rx) = channel(1);

        // Execute should succeed and send lines to channel
        let result = t.execute(String::new(), tx).await;
        assert!(result.is_ok());

        let mut lines: Vec<String> = vec![];
        while let Some(received) = rx.recv().await {
            lines.push(received);
        }
        let res = vec![["line1", "line2", "line3"].join("\n")];
        assert_eq!(lines, res);
    }

    /// Confirms a file above the limit fails without emitting a partial payload.
    #[tokio::test]
    async fn test_read_rejects_an_oversized_file_without_sending_a_partial_payload() {
        use std::io::Write;

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"abcd").unwrap();
        let mut reader = Read::new("test");
        reader.path(file.path().to_str().unwrap());
        reader.max_bytes(NonZeroU32::new(3).unwrap());
        let (tx, mut rx) = channel(1);

        let error = reader.execute(String::new(), tx).await.unwrap_err();
        assert!(matches!(
            error.downcast_ref::<EngineStatus>(),
            Some(EngineStatus::ReadTooLarge { max_bytes: 3, .. })
        ));
        assert!(rx.recv().await.is_none());
    }
}
