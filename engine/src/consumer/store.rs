//! Special task that will store input in a tree of files, one every hour for now.
//!
//! 1. create a directory with the job ID
//! 2. store all data coming from the pipe in files every hour
//!
//! FIXME: make it configurable?
//!
//! This module is data-agnostic and does not care whether it is JSON, binary or a CSV.
//!

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use chrono::{Datelike, Timelike, Utc};
use eyre::Result;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{error, trace};

use fetiche_macros::RunnableDerive;

use crate::{Consumer, EngineStatus, Freq, Runnable, IO};

/// Struct describing the data for the `Store` task.
///
/// We currently do not cache the open file for the current output, we might
/// do that in the future but the cost is 2 more syscalls but simplified code.
///
#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Store {
    /// IO Capability
    io: IO,
    /// Our storage directory
    path: PathBuf,
    /// Our rollover strategy
    freq: Freq,
    /// Specific extension, if needed
    ext: Option<String>,
}

impl From<Store> for Consumer {
    fn from(f: Store) -> Self {
        Consumer::Store(f)
    }
}

impl Default for Store {
    fn default() -> Self {
        Store {
            io: IO::Consumer,
            path: PathBuf::from(""),
            freq: Freq::Hourly,
            ext: None,
        }
    }
}

impl Store {
    /// Given a base directory in `path` create the tree if not present and store the full
    /// path as path/ID
    ///
    #[tracing::instrument]
    pub async fn new(path: &str, id: usize, freq: Freq) -> Result<Self> {
        trace!("store::new");

        // Ensure path is defined.
        //
        if path.is_empty() {
            error!("Store: path can not be empty");
            return Err(EngineStatus::NoPathDefined.into());
        }

        // We want to have `path/current` pointing to `path/ID`
        //
        let base = PathBuf::from(path);
        let path = base.join(id.to_string());
        trace!("Store path is {}", path.to_string_lossy().to_string());

        // Base MUST be writable so we create BASE/ID
        //
        if !path.exists() {
            trace!("Store: creating {}", path.to_string_lossy().to_string());

            if let Err(e) = fs::create_dir_all(&path).await {
                let path = path.to_string_lossy().to_string();
                error!("Store: can not create {}: {}", path, e.to_string());
            }
        }

        let curr = base.join("current");
        #[cfg(unix)]
        if curr.exists() {
            if let Err(e) = fs::remove_file(&curr).await {
                let curr = curr.to_string_lossy().to_string();

                error!("Store: can not remove symlink {}: {}", curr, e.to_string());
                return Err(EngineStatus::RemoveLink(curr).into());
            }
        }

        // #[cfg(windows)]
        // if let Err(e) = std::os::windows::fs::symlink_dir(&path, &curr) {
        //     let path = path.to_string_lossy().to_string();
        //     let curr = curr.to_string_lossy().to_string();
        //
        //     error!(
        //         "Store: can not create symlink to {} as {}: {}",
        //         path,
        //         curr,
        //         e.to_string()
        //     );
        //     return Err(EngineStatus::CreateLink(path, curr).into());
        // }

        #[cfg(unix)]
        if let Err(e) = fs::symlink(&path, &curr).await {
            let path = path.to_string_lossy().to_string();
            let curr = curr.to_string_lossy().to_string();

            error!(
                "Store: can not create symlink to {} as {}: {}",
                path,
                curr,
                e.to_string()
            );
            return Err(EngineStatus::CreateLink(path, curr).into());
        }

        Ok(Store {
            io: IO::Consumer,
            path,
            freq,
            ext: None,
        })
    }

    #[tracing::instrument(skip(self))]
    pub fn use_ext(&mut self, ext: &str) -> &mut Self {
        self.ext = Some(ext.to_string());
        self
    }

    /// Store and rotate every hour for now.  We open/create and write every packet without
    /// trying to open first.  More syscalls but these are cheap.
    ///
    #[tracing::instrument(skip(self, _stdout))]
    pub async fn execute(&mut self, data: String, _stdout: Sender<String>) -> Result<()> {
        trace!("store::execute");

        let tm = Utc::now();

        // Extract parts to create a filename
        //
        // file name format is YYYYMMDD-HH0000 (or -000000 for daily rollover)
        //
        let fname = match self.freq {
            Freq::Daily => {
                format!("{}{:02}{:02}-000000", tm.year(), tm.month(), tm.day())
            }
            Freq::Hourly => {
                format!(
                    "{}{:02}{:02}-{:02}0000",
                    tm.year(),
                    tm.month(),
                    tm.day(),
                    tm.hour()
                )
            }
        };

        let fname = if let Some(ext) = &self.ext {
            fname + ext
        } else {
            fname
        };

        // Full path is BASE/ID/FNAME
        //
        let fname = self.path.clone().join(fname);

        trace!("final name={}", fname.to_string_lossy().to_string());

        // Append to it (and create if not yet present)
        //
        let mut fh = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(fname)
            .await?;

        fh.write_all(data.as_bytes()).await?;
        fh.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::mpsc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_store_new() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let store = Store::new(path, 1, Freq::Hourly).await;
        assert!(store.is_ok());

        let store = store.unwrap();
        assert_eq!(store.freq, Freq::Hourly);
        assert_eq!(store.io, IO::Consumer);
        assert!(store.ext.is_none());
    }

    #[tokio::test]
    async fn test_store_use_ext() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path, 1, Freq::Hourly).await.unwrap();
        store.use_ext(".json");

        assert_eq!(store.ext, Some(".json".to_string()));
    }

    #[tokio::test]
    async fn test_store_execute() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path, 1, Freq::Hourly).await.unwrap();
        let (tx, _rx) = mpsc::channel();

        let result = store.execute("test data".to_string(), tx).await;
        assert!(result.is_ok());

        let tm = Utc::now();
        let fname = store.path.join(format!(
            "{}{:02}{:02}-{:02}0000",
            tm.year(),
            tm.month(),
            tm.day(),
            tm.hour()
        ));
        assert_eq!(std::fs::exists(fname.to_str().unwrap()).unwrap(), true);
    }

    #[tokio::test]
    async fn test_store_execute_ext() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let mut store = Store::new(path, 1, Freq::Hourly).await.unwrap();
        store.use_ext(".json");
        let (tx, _rx) = mpsc::channel();

        // Ensure we have the tree
        //
        assert!(std::fs::exists(&store.path).unwrap());
        assert!(std::fs::exists(&dir.path().join(Path::new("1"))).unwrap());

        #[cfg(unix)]
        assert!(std::fs::exists(&dir.path().join(Path::new("current"))).unwrap());

        let result = store.execute("test data".to_string(), tx).await;
        assert!(result.is_ok());

        let tm = Utc::now();
        let fname = store.path.join(format!(
            "{}{:02}{:02}-{:02}0000.json",
            tm.year(),
            tm.month(),
            tm.day(),
            tm.hour()
        ));
        assert_eq!(std::fs::exists(fname.to_str().unwrap()).unwrap(), true);
    }
}
