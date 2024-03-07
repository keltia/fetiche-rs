use dateparser::parse;

fn main() {
    let a = parse("2024-77-01").unwrap();
    dbg!(&a);
}
