use eyre::Result;
use std::env;
use std::fs::canonicalize;
use std::path::{absolute, PathBuf};

fn main() -> Result<()> {
    let fname = env::args().nth(1).ok_or(eyre::eyre!("missing output path"))?;
    println!("input path: {:?}", fname);

    let name = match canonicalize(&fname) {
        Ok(fname) => fname,
        Err(err) => {
            PathBuf::from(err.to_string())
        }
    };
    println!("canonicalized path: {:?}", name);

    let fname = match absolute(&fname) {
        Ok(fname) => fname,
        Err(err) => {
            PathBuf::from(err.to_string())
        }
    };
    println!("absolute path: {:?}", fname);


    Ok(())
}