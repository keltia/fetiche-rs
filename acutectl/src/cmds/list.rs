use std::path::PathBuf;

use anyhow::Result;

use fetiche_sources::Sources;
use format_specs::Format;

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
    let str = cfg
        .iter()
        .map(|(name, site)| format!("{} = {}", name, site))
        .collect::<Vec<_>>();
    Ok(str.join("\n"))
}
