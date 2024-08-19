use fetiche_common::Versioned;
use fetiche_macros::{add_version, into_configfile};
use serde::Deserialize;

#[add_version(2)]
#[derive(Debug, Default)]
struct Foo {
    pub name: String,
}

// Specify version and filename.
//
#[into_configfile(version = 3, filename = "bar.hcl")]
#[derive(Debug, Default, Deserialize)]
struct Bar {
    pub value: u32,
}

// Use default defined in the macro.
//
#[into_configfile]
#[derive(Debug, Default, Deserialize)]
struct Baz {
    pub data: u32,
}


fn main() {
    let foo = Foo::new();
    let bar = Bar::new();
    let toto = Baz::new();

    dbg!(&foo);
    assert_eq!(2, foo.version());
    println!("struct Foo version is {}", foo.version());

    dbg!(&bar);
    assert_eq!(3, bar.version());
    assert_eq!("bar.hcl", bar.filename());
    println!("struct Bar version is {} from {}", bar.version(), bar.filename());

    dbg!(&toto);
    assert_eq!(1, toto.version());
    assert_eq!("config.hcl", toto.filename());
    println!("struct Baz version is {} from {}", toto.version(), toto.filename());
}
