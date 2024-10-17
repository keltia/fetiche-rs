//! Avionix module.
//!
//! This module is for the Avionix Cube antenna API which supports only streams.
//!
//! There are one trait implementation:
//! - `Streamable`
//!
//! There are two options here:
//! - HTTP call on usual TLS port, not more than 1 call/s with a 5s window
//! - streaming JSONL records by connecting to port 50007
//!
//! We implement the 2nd one as it is simpler and does not need any cache..
//!

pub use cube::*;
pub use server::*;

use std::fmt::Display;
use std::io::{Read, Write};
use std::str::FromStr;

/// This is the code to access the TCP streaming port on a given antenna
mod cube;
/// This is the code used when accessing the Avionix API or TCP streaming server
mod server;

/// Buffer size
const BUFSIZ: usize = 65_536;
