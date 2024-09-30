use fetiche_engine::TokenStorage;

fn main() {
    #[cfg(windows)]
    let storage = TokenStorage::register("/Users/roberto/AppData/Local/drone-utils/tokens");

    #[cfg(unix)]
    let storage = TokenStorage::register("/Users/roberto/.config/drone-utils/tokens");

    dbg!(&storage);
}
