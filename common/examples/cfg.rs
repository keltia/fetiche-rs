//! Example for `ConfigEngine`
//!

use eyre::Result;
use serde::Deserialize;

use fetiche_common::{init_logging, ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;

const CVERSION: usize = 1;

#[into_configfile]
#[derive(Clone, Default, Debug, Deserialize)]
struct Foo {
    pub _name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("cfg", false)?;

    let base = directories::BaseDirs::new().unwrap();
    dbg!(&base);
    let b = ConfigFile::<Foo>::load(Some("examples/local.hcl"))?;
    dbg!(&b);
    let b = b.inner().unwrap();

    assert_eq!(CVERSION, b.version());
    Ok(())
}
