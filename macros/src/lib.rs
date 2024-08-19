use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, LitInt};

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
                    ::tracing::trace!("Runnable({})", stringify!(#klass));

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

/// Add a `version(usize)` with to any given `struct` and implement the `Versioned`trait for it
///
#[proc_macro_attribute]
pub fn add_version(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse::<LitInt>(args)
        .unwrap_or_else(|_| proc_macro2::Literal::usize_unsuffixed(1).into());
    let mut input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;

    let version_ident = Ident::new("version", ident.span());
    let version_type = quote! { usize };

    let output = match input.data {
        Data::Struct(ref mut data_struct) => {
            if let Fields::Named(fields) = &mut data_struct.fields {
                fields.named.push(
                    syn::Field::parse_named
                        .parse2(quote! { #version_ident: #version_type })
                        .unwrap(),
                )
            }

            quote! {
                #input

                impl Versioned for #ident {
                    fn version(&self) -> #version_type {
                        self.version
                    }
                }

                impl #ident {
                    pub fn new() -> Self {
                        Self {
                            version: #args,
                            ..Default::default()
                        }
                    }
                }
            }
        }
        _ => panic!("#[add_version)] is only for struct with named fields"),
    };
    output.into()
}
