use eyre::Result;
use strum::{strum::VariantNames, EnumString};

#[derive(Debug, strum::Display, strum::VariantNames, EnumString)]
enum Cmd<'a> {
    Bare,
    String(&'a str),
    Complex { foo: usize },
}

fn main() -> Result<()> {
    let a = Cmd::Bare;
    let b = Cmd::String("here is a string");
    let c = Cmd::Complex { foo: 42 };

    println!("a={a} b={b} c={c} c={c:?}");

    Ok(())
}
