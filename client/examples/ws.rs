//! Management of workspace directories for Engine instances.
//!
//! This module provides functionality for handling workspace directories that are used by
//! different Engine instances. It includes:
//!
//! - Managing a local filesystem backend for workspace operations
//! - Creating and initializing workspace directories
//! - Listing existing workspace directories that follow the naming convention
//! - Deleting workspace directories and their contents
//! - Automatic cleanup of temporary files
//!
//! # Directory Structure
//!
//! Each workspace directory exists under a common base directory and is identified by
//! a numeric suffix. For example:
//!
//! ```text
//! /base_dir/
//!   ├── 001/
//!   ├── 002/
//!   └── 003/
//! ```
//!
//! # Usage Example
//!
//! ```no_run
//! use std::path::PathBuf;
//! use engine::workspace::WorkSpace;
//!
//! # async fn run() -> eyre::Result<()> {
//! // Create a new workspace with base directory
//! let workspace = WorkSpace::new(&PathBuf::from("/tmp/workspaces"))?;
//!
//! // List all workspace directories
//! let dirs = workspace.list().await;
//!
//! // Delete a specific workspace
//! workspace.delete("001").await?;
//! # Ok(())
//! # }
//! ```
//!
//! The module automatically handles temporary file cleanup and provides a safe way
//! to manage workspace resources across multiple Engine instances.
//!

use std::fmt::{Debug, Display};
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, vec};

use eyre::Result;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::ObjectStore;
use regex::Regex;
use strum::EnumString;
use sysinfo::{Pid, System};
use tracing::{error, trace};

use crate::Client;
use fetiche_engine::WsError;
// -----

/// File created into our workdir to indicate something is running.
const CANARY_FILE: &str = "running";

/// Magic file used to identify workspace directories.
const WS_MAGIC: &str = "FETICHE_WS";

// -----

/// Represents the current status of a workspace directory.
///
/// This enum indicates whether a workspace is currently in use
/// or has been abandoned/left in an inconsistent state.
///
#[derive(Clone, Copy, Debug, EnumString, strum::Display)]
pub enum WsStatus {
    /// Indicates that the workspace is currently active and in use
    Running,
    /// Indicates that the workspace is inactive or abandoned
    Stale,
    /// Indicates that data is present in the directory
    Data,
}

/// Represents a workspace directory item with its path and status.
///
#[derive(Clone, Debug)]
pub struct WsItem {
    /// Path to the workspace item
    path: PathBuf,
    /// Current status of the workspace item
    status: Vec<WsStatus>,
    /// files in item directory
    size: usize,
}

impl WsItem {
    #[tracing::instrument]
    pub fn new_from_path(path: &PathBuf) -> Result<Self> {
        if path.is_dir() {
            let canary = path.join(CANARY_FILE);
            trace!("checking {}", canary.to_string_lossy().to_string());

            // Check whether process is still running
            //
            let mut state = if canary.exists() {
                // File exists, but is the process still running?
                //
                let pid_str = path.file_name().unwrap().to_string_lossy().to_string();

                // Use sysinfo to check existence safely on both Windows and Unix
                //
                let mut system = System::new_all();
                system.refresh_all();

                let is_running = if let Ok(pid_num) = pid_str.parse::<usize>() {
                    system.process(Pid::from(pid_num)).is_some()
                } else {
                    false
                };

                if is_running {
                    vec![WsStatus::Running]
                } else {
                    vec![WsStatus::Stale]
                }
            };

            // Check if there is also data in the directory
            //
            let files = fs::read_dir(path.clone())?;
            let mut count = 0;
            for file in files {
                let file = file?;
                let file_path = file.path();
                if file_path.file_name().unwrap().to_string_lossy() == CANARY_FILE {
                    continue;
                }
                if file_path.is_file() {
                    state.push(WsStatus::Data);
                    count += 1;
                }
            }
            trace!("{} files in {}", count, path.to_string_lossy().to_string());

            Ok(WsItem {
                path: path.clone(),
                status: state,
                size: count,
            })
        } else {
            Err(WsError::NotADirectory(path.to_string_lossy().to_string()).into())
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn state(&self) -> Vec<WsStatus> {
        self.status.clone()
    }

    #[tracing::instrument(skip(self))]
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Display for WsItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = match self.size() {
            0 => "".to_string(),
            1 => "1 file".to_string(),
            n => format!("{} files", n),
        };
        let entry = self.path.file_name().unwrap().to_string_lossy();
        let state = self.state().join(", ");
        write!(f, "{} ({}), {}", entry, state, size)
    }
}

// -----

/// Manages workspace directories for Engine instances.
///
/// WorkSpace handles creation, listing and deletion of workspace directories
/// using a local filesystem backend. It maintains a base directory path and
/// provides operations for managing subdirectories within it.
///
#[derive(Clone, Debug)]
pub struct Workspace {
    /// Path to the directory
    basedir: PathBuf,
    /// Object storage instance
    dir: Arc<LocalFileSystem>,
    /// Currently loaded items
    items: Option<Vec<WsItem>>,
}

impl Workspace {
    /// Creates a new WorkSpace instance with the given base directory path.
    ///
    /// The function initializes a workspace in the specified base directory. If the directory
    /// does not exist, it will be created. The workspace is configured with automatic
    /// cleanup of temporary files enabled.
    ///
    /// # Arguments
    ///
    /// * `base` - Path to the base directory for the workspace
    ///
    /// # Returns
    ///
    /// Returns a Result containing either:
    /// * A new WorkSpace instance if successful
    /// * An error if the directory cannot be created or accessed
    ///
    #[tracing::instrument]
    pub fn create(basedir: &PathBuf) -> Result<Self, WsError> {
        if !basedir.exists() {
            fs::create_dir_all(basedir)
                .map_err(|_| WsError::CannotCreate(basedir.to_string_lossy().to_string()))?;
        }

        // Create our magic marker
        //
        let magic = basedir.join(WS_MAGIC);
        let _ = File::create(magic)
            .map_err(|_| WsError::CannotCreateMagic(basedir.to_string_lossy().to_string()))?;

        let dir = LocalFileSystem::new_with_prefix(basedir)
            .map_err(|_| WsError::CannotCreate(basedir.to_string_lossy().to_string()))?
            .with_automatic_cleanup(true);
        Ok(Self {
            basedir: basedir.clone(),
            dir: dir.into(),
            items: None,
        })
    }

    /// Returns the base directory path of this workspace.
    ///
    #[tracing::instrument(skip(self))]
    pub fn path(&self) -> PathBuf {
        self.basedir.clone()
    }

    /// Lists all valid workspace directories.
    ///
    /// Returns a sorted vector of directory paths that match the workspace format
    /// (ending with numeric identifiers). Only includes directories, not files.
    ///
    #[tracing::instrument]
    pub async fn load(basedir: &PathBuf) -> Result<Self> {
        // Check our magic file.
        //
        let magic = basedir.join(WS_MAGIC);
        if !magic.exists() {
            return Err(WsError::NotAWorkspace(basedir.to_string_lossy().to_string()).into());
        }

        // Create a regex to match directory names with only numbers (aka PIDs)
        //
        let dir_re = Regex::new(r"/(\d+)$")?;

        let mut dirs = vec![];
        let base = match fs::read_dir(basedir.clone()) {
            Ok(base) => base,
            Err(e) => {
                error!("error={e}");
                return Err(e.into());
            }
        };
        for entry in base {
            if let Ok(entry) = entry {
                let entry_str = entry.path().to_string_lossy().to_string();
                trace!(entry = { &entry_str });

                if entry.path().is_dir() && dir_re.is_match(&entry_str) {
                    let item = WsItem::new_from_path(&entry.path())?;
                    dirs.push(item);
                }
            }
        }
        let ws = Workspace {
            basedir: basedir.clone(),
            dir: Arc::new(LocalFileSystem::new_with_prefix(basedir)?.with_automatic_cleanup(true)),
            items: Some(dirs),
        };
        Ok(ws)
    }

    #[tracing::instrument(skip(self))]
    pub async fn attach(&mut self, client: Client) -> Result<()> {}

    #[tracing::instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<WsItem>> {
        Ok(self.items.clone().unwrap_or(vec![]))
    }

    /// Removes all contents of the specified directory within the workspace.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn delete<T>(&self, dir: T) -> Result<()>
    where
        T: Into<Path> + Debug,
    {
        let fs = self.dir.clone();
        let canary: Path = dir.into();
        Ok(fs.delete(&canary).await?)
    }
}

#[tokio::main]
async fn main() {
    let ws = Workspace::load("/Users/Acute/var/run").await?;
    println!("{:?}", ws);
}
