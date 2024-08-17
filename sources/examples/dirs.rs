use directories::ProjectDirs;
use tracing::error;

fn main() {
    let p = ProjectDirs::from("", "", "drone-utils");
    match p {
        Some(p) => println!("path = {p:?}"),
        None => panic!("grr"),
    }

    let homedir = std::env::var("LOCALAPPDATA")
        .map_err(|e| error!("No LOCALAPPDATA variable defined, can not continue"))
        .unwrap();
    println!("home={homedir}");

    let config = fetiche_sources::Sources::config_path();
    println!("config={config:?}");
}
