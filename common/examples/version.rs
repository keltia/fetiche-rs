use fetiche_common::Versioned;
use fetiche_macros::add_version;

#[add_version(2)]
#[derive(Debug, Default)]
pub struct Foo {
    pub name: String,
}

fn main() {
    let foo = Foo::new();

    assert_eq!(2, foo.version());
    println!("struct Foo version is {}", foo.version());
}
