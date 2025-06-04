//! This is a task module that act like the UNIX command of the same name:
//! copy whatever it receives into a file and pass the data down the pipe
//! unchanged
//!

use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::mpsc::Sender;

use eyre::Result;
use tracing::trace;

use fetiche_macros::RunnableDerive;

use crate::{Middle, Runnable, IO};
#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Tee {
    io: IO,
    pub fname: String,
}

impl From<Tee> for Middle {
    fn from(t: Tee) -> Self {
        Middle::Tee(t.clone())
    }
}

impl Tee {
    #[inline]
    #[tracing::instrument]
    pub fn into(p: &str) -> Self {
        Tee {
            io: IO::Filter,
            fname: p.to_string(),
        }
    }

    /// This is the main task.  Every data packet we receive will be written in the designed
    /// file then passed down.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("tee::execute");
        let mut fh = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.fname)?;
        write!(fh, "{data}")?;
        fh.flush()?;
        Ok(stdout.send(data)?)
    }
}

impl Default for Tee {
    fn default() -> Self {
        Self {
            io: IO::Filter,
            fname: "".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use std::sync::mpsc;

    use tempfile::tempdir;

    #[tokio::test]
    async fn test_tee_create_and_write() {
        // Create a temporary directory for the test file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_output.txt");

        // Create a Tee instance
        let mut tee = Tee::into(file_path.to_str().unwrap());

        // Mock a channel to simulate stdout behavior
        let (tx, rx) = mpsc::channel();

        // Write some data using the Tee instance
        let data = "Hello, Tee!".to_string();
        tee.execute(data.clone(), tx).await.unwrap();

        // Check that the data was sent through the channel
        assert_eq!(rx.recv().unwrap(), data);

        // Check that the data was written to the file
        let mut file = File::open(file_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, data);
    }

    #[tokio::test]
    async fn test_tee_multiple_writes() {
        // Create a temporary directory for the test file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_output_multiple.txt");

        // Create a Tee instance
        let mut tee = Tee::into(file_path.to_str().unwrap());

        // Mock a channel to simulate stdout behavior
        let (tx, rx) = mpsc::channel();

        // Write multiple pieces of data using the Tee instance
        let data1 = "First line\n".to_string();
        let data2 = "Second line\n".to_string();

        tee.execute(data1.clone(), tx.clone()).await.unwrap();
        tee.execute(data2.clone(), tx).await.unwrap();

        // Collect the received data from the channel
        let outputs: Vec<_> = rx.try_iter().collect();
        assert_eq!(outputs, vec![data1.clone(), data2.clone()]);

        // Check that the data was written to the file
        let mut file = File::open(file_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, format!("{data1}{data2}"));
    }
}
