//! Example demonstrating path canonicalization and absolute path resolution in Rust.
//!
//! This example shows how to:
//! - Convert relative paths to canonical form using `canonicalize`
//! - Convert relative paths to absolute form using `absolute`
//! - Handle errors during path conversions
//!
//! Usage: cargo run --example canon <path>
//!
use eyre::Result;
use std::env;
use std::fs::canonicalize;
use std::path::{absolute, PathBuf};

/// Demonstrates path canonicalization and absolute path resolution.
///
/// # Returns
/// - `Result<()>` - Ok if path conversions succeeded, Err with error details otherwise
///
/// # Errors
/// Returns an error if no path argument is provided or if path conversions fail
///
fn main() -> Result<()> {
    // Get the input path from command line arguments
    //
    let fname = env::args().nth(1).ok_or(eyre::eyre!("missing output path"))?;
    println!("input path: {:?}", fname);

    // Convert to canonical path (resolves symlinks and normalizes path)
    //
    let name = canonicalize(&fname).unwrap_or_else(|err| {
        PathBuf::from(err.to_string())
    });
    println!("canonicalized path: {:?}", name);

    // Convert to absolute path (does not resolve symlinks)
    //
    let fname = absolute(&fname).unwrap_or_else(|err| {
        PathBuf::from(err.to_string())
    });
    println!("absolute path: {:?}", fname);

    Ok(())
}