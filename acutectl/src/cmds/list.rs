use anyhow::Result;

use fetiche_formats::Format;
use fetiche_sources::{Auth, Sources};

/// Fetch the list of supported formats and their description.
///
pub fn list_formats() -> Result<String> {
    let str = Format::list()?;
    Ok(str)
}

/// Fetch all the different sources available.
///
/// TODO: we need a Sites::list() like for formats-specs above.
///
pub fn list_sources(cfg: &Sources) -> Result<String> {
    let str = cfg.list()?;
    Ok(str)
}
