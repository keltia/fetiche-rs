//! Demo using `serde_with`  to automatically convert the strings returned by Flightaware API into
//! their proper type.
//!
//! Running this gives:
//! ```text
//!     Finished dev [unoptimized + debuginfo] target(s) in 1.61s
//      Running `/Users/roberto/Src/Rust/src/fetiche-rs/target/debug/examples/fa`
// [formats/examples/fa.rs:18] r = Position {
//     lat: 44.8,
//     lon: Some(
//         -6.7,
//     ),
// }
//! ```

use eyre::Result;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct Position {
    #[serde_as(as = "DisplayFromStr")]
    pub lat: f32,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub lon: Option<f32>,
}

fn main() -> Result<()> {
    let r: Position = serde_json::from_str("{\"lat\": \"44.8\", \"lon\": \"-6.7\"}")?;

    dbg!(r);
    Ok(())
}
