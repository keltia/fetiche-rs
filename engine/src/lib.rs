//! Library part of the engined binary
//!

const NAME: &str = env!("CARGO_PKG_NAME");
const EVERSION: &str = env!("CARGO_PKG_VERSION");

pub fn version() -> String {
    format!("{}/{}", NAME, EVERSION)
}
