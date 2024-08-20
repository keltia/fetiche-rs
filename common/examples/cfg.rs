//! Example for `ConfigEngine`
//!

use eyre::Result;
use serde::Deserialize;
use std::path::Path;

use fetiche_common::{init_logging, ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;

const CVERSION: usize = 1;

#[into_configfile]
#[derive(Clone, Default, Debug, Deserialize)]
struct Foo {
    pub name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging("cfg", false)?;

    let base = directories::BaseDirs::new().unwrap();
    dbg!(&base);
    let a = ConfigFile::<Foo>::load(Some("examples/local.hcl"))?;
    dbg!(&a);
    let b = a.inner();
    println!("{:?}", a.list());

    assert_eq!(CVERSION, b.version());

    // wrong type for default "config.hcl"
    //
    let c = ConfigFile::<Foo>::load(None);
    assert!(c.is_err());

    // no "local.hcl" in basedir
    //
    let c = ConfigFile::<Foo>::load(Some("local.hcl"));
    assert!(c.is_err());

    assert!(Path::new("foo").is_relative());
    assert!(Path::new("../foo").is_relative());
    assert!(Path::new("./foo").is_relative());

    assert!(Path::new("/foo").is_absolute());
    Ok(())
}
