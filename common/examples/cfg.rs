//! Example for `ConfigEngine`
//!

use eyre::Result;
use serde::Deserialize;

use fetiche_common::{init_logging, ConfigEngine, Versioned};

const CVERSION: usize = 1;

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

    let b: Foo = ConfigEngine::load(Some("examples/local.hcl"))?;
    dbg!(&b);

    assert_eq!(CVERSION, b.version());
    Ok(())
}
