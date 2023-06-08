//! Module handling the conversions between different formats
//!

use std::io::Write;

use fetiche_formats::Format;

use crate::Runnable;

#[derive(Debug)]
pub struct Into {
    pub from: Format,
    pub into: Format,
}

impl Runnable for Into {
    fn run(&mut self, out: &mut dyn Write) -> anyhow::Result<()> {
        todo!()
    }
}
