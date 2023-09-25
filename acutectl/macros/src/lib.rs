use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Most basic proc_macro ever: use as a template.
///
/// `execute()` takes whatever was sent from the previous stage and process is, knowing that
/// any input should be sent directly to the stdout channel.
///
#[proc_macro_derive(RunnableDerive)]
pub fn runnable(input: TokenStream) -> TokenStream {
    let klass = parse_macro_input!(input as DeriveInput);
    let klass = klass.ident;
    let outer = quote!(
        impl Runnable for #klass {
            fn cap(&self) -> IO {
                self.io.clone()
            }

            fn run(
                &mut self,
                input: ::std::sync::mpsc::Receiver<::std::string::String>,
            ) -> (::std::sync::mpsc::Receiver<String>, ::std::thread::JoinHandle<Result<()>>) {
                let (stdout, stdin) = ::std::sync::mpsc::channel::<::std::string::String>();

                let mut src = self.clone();
                let h = ::std::thread::spawn(move || {
                    ::log::trace!("Runnable({})", stringify!(#klass));

                    // Add our message
                    //
                    for data in input {
                        // Do something (or not) with the input data if there is an error
                        //
                        src.execute(data, stdout.clone()).unwrap();
                    }
                    Ok(())
                });
                (stdin, h)
            }
        }
    );
    outer.into()
}
