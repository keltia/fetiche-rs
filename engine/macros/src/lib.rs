use proc_macro::TokenStream;

use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// Most basic proc_macro ever: use as a template.
///
#[proc_macro_derive(RunnableDerive)]
pub fn runnable(input: TokenStream) -> TokenStream {
    let klass = parse_macro_input!(input as DeriveInput);
    let klass = klass.ident;
    let outer = quote!(
        impl Runnable for #klass {
        fn run(
            &mut self,
            input: ::std::sync::mpsc::Receiver<::std::string::String>,
        ) -> (::std::sync::mpsc::Receiver<String>, ::std::thread::JoinHandle<Result<()>>) {
            let (stdout, stdin) = ::std::sync::mpsc::channel::<::std::string::String>();

            let h = ::std::thread::spawn(move || {
                ::log::trace!("Runnable({})", stringify!(#klass));

                // Add our message
                //
                for data in input {
                    // Do something (or not) with the input data if there is an error
                    //
                    let data = self.transform(data).unwrap();
                    if stdout.send(data).is_err() {
                        break;
                    }
                }
                Ok(())
            });
            (stdin, h)
        }
    });
    outer.into()
}

