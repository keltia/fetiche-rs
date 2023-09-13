use std::path::PathBuf;

/// Default working directory (UNIX)
#[cfg(unix)]
pub(crate) const DEF_HOMEDIR: &str = "/var/run/fetiche";

/// Returns the path of the default working directory file. On Unix systems we use `/var/db/fetiche`
/// as we want persistence.  `/var/run` is often a ramdisk or something cleaned up on reboot.
///
#[cfg(unix)]
#[tracing::instrument]
pub(crate) fn default_workdir() -> Result<PathBuf> {
    let def = match std::env::var("FETICHE_WORKDIR") {
        Ok(path) => PathBuf::from(path),
        None => {
            let workdir = PathBuf::from(DEF_HOMEDIR);

            // Create if not existing.
            //
            if !workdir.exists() {
                create_dir_all(DEF_HOMEDIR)?;
            }
            workdir
        }
    };
    Ok(def)
}

/// Returns the path of the default config file.  Here we use the standard %LOCALAPPDATA%
/// variable to base our directory into.
///
#[cfg(windows)]
#[tracing::instrument]
pub(crate) fn default_workdir() -> eyre::Result<PathBuf> {
    let def = match std::env::var("FETICHE_WORKDIR") {
        Ok(path) => PathBuf::from(path),
        Err(_) => {
            let local = std::env::var("LOCALAPPDATA")?;

            let def: PathBuf = [PathBuf::from(local), PathBuf::from("fetiche")]
                .iter()
                .collect();

            // Create if not existing.
            //
            if !def.exists() {
                let _ = std::fs::create_dir_all(&def)?;
            }
            def
        }
    };
    Ok(def)
}
