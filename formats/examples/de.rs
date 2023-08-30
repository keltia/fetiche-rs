use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use strum::EnumString;

#[derive(Clone, Debug, Deserialize, EnumString, strum::Display, Serialize)]
#[serde(untagged)]
pub enum Type {
    Integer(i32),
    Str(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Foo {
    pub tm: Type,
}

fn main() -> Result<()> {
    let i = Type::Integer(1687363200);

    let str = serde_json::to_string(&i)?;
    println!("{}", str);

    let f = Foo { tm: i };
    let str = serde_json::to_string(&f)?;
    println!("{}", str);

    let str = r##"{"tm": "1687363200"}"##;

    let a: Foo = from_str(str)?;

    dbg!(a);
    Ok(())
}
