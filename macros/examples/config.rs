//use fetiche_macros::into_configfile;

trait Versioned {
    fn version(&self) -> usize;
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(into_configfile))]
struct ConfigArgs {
    #[darling(default)]
    version: usize,
    #[darling(default)]
    filename: String,
}

use darling::FromDeriveInput;
use syn::parse_quote;

fn main() {
    //let foo = Bar::new();

    let input = ConfigArgs::from_derive_input(&parse_quote! {
        #[into_configfile(version = 1, filename = "foo.hcl")]
        struct Foo;
    }).unwrap();

    println!("{:#?}", input);
}
