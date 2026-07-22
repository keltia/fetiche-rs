//use fetiche_macros::into_configfile;

#[allow(dead_code)]
trait Versioned {
    fn version(&self) -> usize;
}

#[allow(dead_code)]
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(into_configfile))]
struct ConfigArgs {
    version: usize,
    filename: String,
}

use darling::FromDeriveInput;
use syn::parse_quote;

fn main() {
    //let foo = Bar::new();

    let input = ConfigArgs::from_derive_input(&parse_quote! {
        #[into_configfile(version = 1, filename = "foo.hcl")]
        struct Foo;
    })
    .unwrap();

    println!("{:#?}", input);
}
