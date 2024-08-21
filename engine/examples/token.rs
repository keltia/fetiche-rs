use fetiche_engine::TokenStorage;

fn main() {
    let storage = TokenStorage::register("/Users/roberto/.config/drone-utils/tokens");
    dbg!(&storage);
}
