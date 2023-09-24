//! Actor version of the storage part of fetiched.
//!

mod core;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use actix::prelude::*;
use actix::{Actor, Context, Message};
use eyre::Result;
use serde::Deserialize;
use tokio::fs::File;
use tracing::{error, info, trace};

pub use core::*;

/// Configuration file version
const STORAGE_VERSION: usize = 1;

// ----- Messages

#[derive(Debug, Message)]
#[rtype(result = "Result<String>")]
pub struct StorageList;

#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct StorageInit;

#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct StoreFile;

#[derive(Debug, Message)]
#[rtype(result = "Result<Vec<String>>")]
pub struct ListFiles;

#[derive(Debug, Message)]
#[rtype(result = "Result<String>")]
pub struct RetrieveFile;

#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct StreamFile;

// ----- Actor

#[derive(Clone, Debug, Deserialize)]
pub struct StorageConfig {
    /// Usual check for malformed file
    pub version: usize,
    /// Base directory
    pub basedir: PathBuf,
    /// List of storage types
    pub storage: BTreeMap<String, StorageArea>,
}

#[derive(Debug)]
pub struct StorageActor {
    /// Storage areas
    pub areas: Storage,
    /// Open files
    pub ofiles: Vec<File>,
}

impl StorageActor {
    #[tracing::instrument]
    pub fn new(fname: &str) -> Self {
        trace!("storageactor::new({})", fname);

        let data = fs::read_to_string(PathBuf::from(fname)).unwrap();
        let cfg: StorageConfig = match hcl::from_str(&data) {
            Ok(cfg) => cfg,
            Err(e) => {
                panic!("Invalid {} file: {}", fname, e.to_string());
            }
        };

        if cfg.version != STORAGE_VERSION {
            panic!("Bad version in {}: {} required.", fname, STORAGE_VERSION);
        }

        let areas = Storage::register(&cfg.storage);
        trace!("{} areas loaded", areas.len());
        Self {
            areas,
            ofiles: vec![],
        }
    }
}

impl Actor for StorageActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("Storage is alive");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        info!("Storage is stopped");
    }
}
