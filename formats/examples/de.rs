use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use serde_with::{serde_as, serde_conv};
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

serde_conv!(
    FloatAsInt,
    u32,
    |x: &u32| *x as f64,
    |value: f64| -> Result<_, std::convert::Infallible> {
        Ok(value as u32)
    }
);

#[serde_as]
#[derive(Debug, Deserialize)]
struct Bar {
    #[serde_as(as = "FloatAsInt")]
    pub trk: u32,
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

    let str = r##"{"trk": 42.3765}"##;
    let b: Bar = from_str(str)?;
    dbg!(&b);

    let str = r##"{"trk": 666}"##;
    let c: Bar = from_str(str)?;
    dbg!(&c);

    Ok(())
}
