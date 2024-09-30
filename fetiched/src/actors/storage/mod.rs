//! Actor version of the storage part of fetiched.
//!

use std::collections::BTreeMap;
use std::env::set_current_dir;
use std::fs;
use std::path::PathBuf;

use actix::dev::{MessageResponse, OneshotSender};
use actix::prelude::*;
use actix::{Actor, Context, Message};
use eyre::Result;
use serde::Deserialize;
use tokio::fs::File;
use tracing::{info, trace};

use crate::{response_for, EngineConfig};
pub use core::*;
use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;

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

#[into_configfile(version = 2, filename = "engine.hcl")]
#[derive(Clone, Debug, Deserialize)]
pub struct StorageConfig {
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
        let root = ConfigFile::<StorageConfig>::load(Some(fname))?;
        let cfg = root.inner();
        let home = root.config_path();

        // Move ourselves there
        //
        trace!("workdir={:?}", workdir);
        let _ = set_current_dir(&workdir);

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
