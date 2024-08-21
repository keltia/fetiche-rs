use directories::ProjectDirs;
use fetiche_common::ConfigFile;
use fetiche_sources::Sources;
use tracing::error;

fn main() -> eyre::Result<()> {
    let p = ProjectDirs::from("", "", "drone-utils");
    match p {
        Some(p) => println!("path = {p:?}"),
        None => panic!("grr"),
    }

    let homedir = std::env::var("LOCALAPPDATA")
        .map_err(|e| error!("No LOCALAPPDATA variable defined, can not continue"))
        .unwrap();
    println!("home={homedir}");

    let config = ConfigFile::<Sources>::load(Some("config.hcl"))?;
    println!("basedir = {:?}", config.config_path());
    println!("config={:?}", config.inner());
}
