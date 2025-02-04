use directories::ProjectDirs;
use fetiche_common::ConfigFile;
use fetiche_engine::SourcesConfig;

fn main() -> eyre::Result<()> {
    let p = ProjectDirs::from("", "", "drone-utils");
    match p {
        Some(p) => println!("path = {p:?}"),
        None => panic!("grr"),
    }

    #[cfg(windows)]
    let homedir = std::env::var("LOCALAPPDATA")
        .map_err(|_| eprintln!("No LOCALAPPDATA variable defined, can not continue"))
        .unwrap();

    #[cfg(not(windows))]
    let homedir = std::env::var("HOME")
        .map_err(|_| eprintln!("No HOME variable defined, can not continue"))
        .unwrap();
    println!("home={homedir}");

    let config = ConfigFile::<SourcesConfig>::load(Some("sources.hcl"))?;
    println!("basedir = {:?}", config.config_path());
    println!("config={:?}", config.inner());
    Ok(())
}
