use eyre::Result;
use strum::{EnumString, EnumVariantNames};

#[derive(Debug, strum::Display, EnumVariantNames, EnumString)]
enum Cmd {
    Bare,
    String(String),
    Complex { foo: usize },
}

fn main() -> Result<()> {
    let a = Cmd::Bare;
    let b = Cmd::String("here is a string".to_string());
    let c = Cmd::Complex { foo: 42 };

    println!("a={a} b={b} c={c}");

    Ok(())
}
