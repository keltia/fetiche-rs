use std::path::PathBuf;

use anyhow::Result;

use sources::Sites;

pub fn list_formats() -> Result<String> {
    Ok("None".to_owned())
}

pub fn list_sources(cfg: &Sites) -> Result<String> {
    let str = cfg
        .iter()
        .map(|(name, site)| format!("{} = {}", name, site))
        .collect::<Vec<_>>();
    Ok(str.join("\n"))
}
