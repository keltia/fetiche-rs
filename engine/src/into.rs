//! Module handling the conversions between different formats
//!

use anyhow::Result;

use engine_macros::RunnableDerive;
use fetiche_formats::Format;

use crate::Runnable;

#[derive(Debug, RunnableDerive)]
pub struct Into {
    pub from: Format,
    pub into: Format,
}

impl Into {
    fn transform(&mut self, data: String) -> Result<String> {
        Ok(data)
    }
}
