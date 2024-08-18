use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::{env, fs};

use directories::BaseDirs;
use eyre::{eyre, Result};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use tracing::{debug, error, trace};

use fetiche_common::init_logging;
use fetiche_common::makepath;

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Config filename
const CONFIG: &str = "local.hcl";
/// Current version
const CVERSION: usize = 1;
const TAG: &str = "drone-utils";

pub trait Versioned {
    fn version(&self) -> usize;
}

/// Configuration for the CLI tool, supposed to include parameters and most importantly
/// credentials for the various sources.
///
#[derive(Debug)]
pub struct ConfigEngine<T: Clone + Debug + DeserializeOwned + Versioned> {
    /// Version in the file MUST match `CVERSION`
    tag: String,
    basedir: PathBuf,
    _a: PhantomData<T>,
}

impl<T> ConfigEngine<T>
where
    T: Clone + Debug + DeserializeOwned + Versioned,
{
    #[tracing::instrument]
    fn new(tag: &str) -> Self {
        let base = BaseDirs::new();

        let basedir: PathBuf = match base {
            Some(base) => {
                let base = base.config_local_dir().to_string_lossy().to_string();
                debug!("base = {base}");
                let base: PathBuf = makepath!(base, tag);
                base
            }
            None => {
                #[cfg(windows)]
                let homedir = env::var("LOCALAPPDATA")
                    .map_err(|_| error!("No LOCALAPPDATA variable defined, can not continue"))
                    .unwrap();

                #[cfg(unix)]
                let homedir = std::env::var("HOME")
                    .map_err(|_| error!("No HOME variable defined, can not continue"))
                    .unwrap();

                debug!("base = {homedir}");
                let base: PathBuf = makepath!(homedir, tag);
                base
            }
        };
        ConfigEngine {
            tag: String::from(tag),
            basedir,
            _a: PhantomData,
        }
    }

    /// Returns the path of the default config directory
    ///
    #[tracing::instrument]
    pub fn config_path(&self) -> PathBuf {
        self.basedir.clone()
    }

    /// Returns the path of the default config file
    ///
    #[tracing::instrument]
    pub fn default_file(&self) -> PathBuf {
        let cfg = self.config_path().join(CONFIG);
        dbg!(&cfg);
        cfg
    }

    #[tracing::instrument]
    pub fn load(fname: Option<&str>) -> Result<T> {
        trace!("loading config");

        let cfg = ConfigEngine::<T>::new(TAG);

        dbg!(&fname);
        let fname = match fname {
            Some(fname) => PathBuf::from(fname),
            None => cfg.default_file(),
        };

        let data = fs::read_to_string(fname)?;
        dbg!(&data);

        let data: T = hcl::from_str(&data)?;
        dbg!(&data);

        Ok(data)
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Foo {
    version: usize,
    pub name: String,
}

impl Versioned for Foo {
    fn version(&self) -> usize {
        self.version
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("cfg", false)?;

    let fname = env::args().nth(1);
    let b: Foo = ConfigEngine::load(fname.as_deref())?;
    dbg!(&b);

    assert_eq!(CVERSION, b.version());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configengine_load() -> Result<()> {
        let cfg: Foo = ConfigEngine::load(None)?;
        dbg!(&cfg);
        assert_eq!(2, cfg.version);
        Ok(())
    }
}
