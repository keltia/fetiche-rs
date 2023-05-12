use std::path::PathBuf;

use anyhow::Result;
use format_specs::Format;

use sources::Sites;

/// Fetch the list of supported formats and their description.
///
pub fn list_formats() -> Result<String> {
    let str = Format::list()?;
    Ok(str)
}

/// Fetch all the different sources available.
///
pub fn list_sources(cfg: &Sites) -> Result<String> {
    let str = cfg
        .iter()
        .map(|(name, site)| format!("{} = {}", name, site))
        .collect::<Vec<_>>();
    Ok(str.join("\n"))
}
