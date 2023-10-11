//! Actor version of the storage part of fetiched.
//!

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use actix::dev::{MessageResponse, OneshotSender};
use actix::prelude::*;
use actix::{Actor, Context, Message};
use eyre::Result;
use serde::Deserialize;
use tokio::fs::File;
use tracing::{info, trace};

pub use core::*;

use crate::response_for;

mod core;

/// Configuration file version
const STORAGE_VERSION: usize = 1;

/// Default configuration file name in workdir
const STORAGE_FILE: &str = "storage.hcl";

/// This is the part describing the available storage areas
///
#[derive(Clone, Debug)]
pub struct StorageAreas {
    areas: BTreeMap<String, StorageArea>,
}

// ----- Messages

#[derive(Debug, Message)]
#[rtype(result = "Result<StorageAreas>")]
pub struct StorageList;

impl Handler<StorageList> for StorageActor {
    type Result = Result<StorageAreas>;

    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, _msg: StorageList, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.areas.clone())
    }
}

response_for!(StorageAreas);

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
struct StorageConfig {
    /// Usual check for malformed file
    pub version: usize,
    /// List of storage types
    pub storage: BTreeMap<String, StorageArea>,
}

#[derive(Debug)]
pub struct StorageActor {
    /// Storage areas
    pub areas: StorageAreas,
    /// Open files
    pub ofiles: Vec<File>,
}

impl StorageActor {
    #[tracing::instrument]
    pub fn new(workdir: &PathBuf) -> Self {
        trace!("storageactor::new");

        let fname = workdir.join(STORAGE_FILE);
        let data = fs::read_to_string(PathBuf::from(&fname)).unwrap();
        let cfg: StorageConfig = match hcl::from_str(&data) {
            Ok(cfg) => cfg,
            Err(e) => {
                panic!("Invalid {:?} file: {}", fname, e.to_string());
            }
        };

        if cfg.version != STORAGE_VERSION {
            panic!("Bad version in {:?}: {} required.", fname, STORAGE_VERSION);
        }

        let areas = StorageAreas::register(&cfg.storage);
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
