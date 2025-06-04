use std::env;
use std::fs::canonicalize;
use std::path::absolute;

use eyre::Result;
use tracing::debug;

fn main() -> Result<()> {
    let fname = env::args().nth(1).ok_or(eyre::eyre!("missing output path"))?;
    debug!("output path: {:?}", fname);
    let fname = canonicalize(fname)?;
    let fname = env::current_dir()?.join(fname);
    println!("{}", fname.to_string_lossy().to_string());
    Ok(())
}