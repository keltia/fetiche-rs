use anyhow::Result;

use fetiche_formats::Format;
use fetiche_sources::Sources;

/// Fetch the list of supported formats and their description.
///
#[inline]
pub fn list_formats() -> Result<String> {
    let str = Format::list()?;
    Ok(str)
}

/// Fetch all the different sources available.
///
/// TODO: we need a Sites::list() like for formats-specs above.
///
#[inline]
pub fn list_sources(cfg: &Sources) -> Result<String> {
    let str = cfg.list()?;
    Ok(str)
}

/// List token currently stored
///
/// <source_dependent_token_name>-<email>
///
pub fn list_tokens() -> Result<String> {
    let str = Sources::list_tokens()?;
    Ok(str)
}
