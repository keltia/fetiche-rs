use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::Parser;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, LitInt, Type};

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
            #[::tracing::instrument(skip(self))]
            fn cap(&self) -> IO {
                self.io.clone()
            }

            #[::tracing::instrument(skip(self, input))]
            async fn run(
                &mut self,
                mut input: ::std::sync::mpsc::Receiver<::std::string::String>,
            ) -> (::std::sync::mpsc::Receiver<std::string::String>, ::tokio::task::JoinHandle<Result<()>>) {
                let (stdout, mut stdin) = ::std::sync::mpsc::channel::<::std::string::String>();

                let mut src = self.clone();
                let h = ::tokio::spawn(async move {
                    // Add our message
                    //
                    for data in input {
                        // Do something (or not) with the input data if there is an error
                        //
                        src.execute(data, stdout.clone()).await.unwrap();
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

const DEF_VERSION: usize = 1;
const DEF_FILENAME: &str = "config.hcl";

#[derive(Debug, FromMeta)]
struct ConfigArgs {
    version: Option<usize>,
    filename: Option<String>,
}

impl Default for ConfigArgs {
    fn default() -> Self {
        Self {
            version: Some(DEF_VERSION),
            filename: Some(String::from(DEF_FILENAME)),
        }
    }
}

/// Add a `version(usize)` with to any given `struct` and implement the `Versioned`trait for it
///
#[proc_macro_attribute]
pub fn into_configfile(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse attributes
    //
    let attr_args = NestedMeta::parse_meta_list(args.into()).unwrap_or_else(|_| vec![]);

    // Parse struct
    //
    let mut input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;

    // Now transform the attributes into the actual data
    //
    let args = if attr_args.is_empty() {
        ConfigArgs::default()
    } else {
        ConfigArgs::from_list(&attr_args).unwrap()
    };

    let version_value = args.version.unwrap_or(DEF_VERSION);
    let filename = args.filename.unwrap_or(String::from(DEF_FILENAME));

    // Prepare our substitutions
    //
    let version_ident = Ident::new("version", ident.span());
    let version_type = quote! { usize };
    let filename_ident = Ident::new("filename", ident.span());
    let filename_type = quote! { String };

    // Generate output
    //
    let output = match input.data {
        Data::Struct(ref mut data_struct) => {
            match &mut data_struct.fields {
                Fields::Named(fields) => {
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(quote! { #version_ident: #version_type })
                            .unwrap(),
                    );
                    fields.named.push(
                        syn::Field::parse_named
                            .parse2(quote! {
                                #[serde(skip_deserializing)]
                                #filename_ident: #filename_type
                            })
                            .unwrap(),
                    );
                }
                _ => unimplemented!(),
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
                            version: #version_value,
                            filename: String::from(#filename),
                            ..Default::default()
                        }
                    }
                }

                impl IntoConfig for #ident {
                    fn filename(&self) -> #filename_type {
                        self.filename.clone()
                    }
                }
            }
        }
        _ => panic!("#[into_configfile)] is only for struct with named fields"),
    };
    output.into()
}

/// Auto-generate rkyv-compatible parallel structs with `R` prefix.
///
/// This macro creates a parallel struct optimized for rkyv serialization:
/// - DateTime<Utc> → i64 (timestamp_millis)
/// - Vec<T> → Vec<RT> (nested conversions)
/// - Option<T> → Option<RT> (nested conversions)
/// - Generates bidirectional From implementations
///
/// # Example
/// ```ignore
/// #[derive(RkyvClone, Serialize, Deserialize, Debug)]
/// pub struct FusedData {
///     pub version: String,
///     pub timestamp: DateTime<Utc>,
/// }
/// ```
///
/// Generates:
/// ```ignore
/// #[derive(Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq)]
/// pub struct RFusedData {
///     pub version: String,
///     pub timestamp_millis: i64,
/// }
///
/// impl From<&FusedData> for RFusedData { ... }
/// ```
///
#[proc_macro_derive(RkyvClone, attributes(rkyv_skip, rkyv_rename))]
pub fn rkyv_clone(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let orig_name = &input.ident;
    let rkyv_name = format_ident!("R{}", orig_name);

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("RkyvClone only supports structs with named fields"),
        },
        _ => panic!("RkyvClone only supports structs"),
    };

    // Generate rkyv fields and conversion logic
    let mut rkyv_fields = Vec::new();
    let mut from_orig_fields = Vec::new();
    let mut to_orig_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        // Check for skip attribute
        let skip = field.attrs.iter().any(|attr| {
            attr.path().is_ident("rkyv_skip")
        });

        if skip {
            continue;
        }

        // Handle DateTime<Utc> conversion
        let type_str = quote!(#field_type).to_string();

        if type_str.contains("DateTime") && type_str.contains("Utc") {
            let millis_name = format_ident!("{}_millis", field_name);
            rkyv_fields.push(quote! {
                pub #millis_name: i64
            });
            from_orig_fields.push(quote! {
                #millis_name: value.#field_name.timestamp_millis()
            });
            to_orig_fields.push(quote! {
                #field_name: ::chrono::DateTime::from_timestamp_millis(rkyv_val.#millis_name)
                    .unwrap_or_default()
            });
        } else if is_vec_type(field_type) {
            // Handle Vec<T> - convert to Vec<RT> if needed
            let rkyv_field_type = convert_vec_type(field_type);
            rkyv_fields.push(quote! {
                pub #field_name: #rkyv_field_type
            });

            if needs_conversion(&extract_vec_inner(field_type)) {
                from_orig_fields.push(quote! {
                    #field_name: value.#field_name.iter().map(Into::into).collect()
                });
                to_orig_fields.push(quote! {
                    #field_name: rkyv_val.#field_name.iter().map(Into::into).collect()
                });
            } else {
                from_orig_fields.push(quote! {
                    #field_name: value.#field_name.clone()
                });
                to_orig_fields.push(quote! {
                    #field_name: rkyv_val.#field_name.clone()
                });
            }
        } else if is_option_type(field_type) {
            // Handle Option<T> - convert to Option<RT> if needed
            let rkyv_field_type = convert_option_type(field_type);
            rkyv_fields.push(quote! {
                pub #field_name: #rkyv_field_type
            });

            let inner = extract_option_inner(field_type);
            if needs_conversion(&inner) {
                from_orig_fields.push(quote! {
                    #field_name: value.#field_name.as_ref().map(Into::into)
                });
                to_orig_fields.push(quote! {
                    #field_name: rkyv_val.#field_name.as_ref().map(Into::into)
                });
            } else {
                from_orig_fields.push(quote! {
                    #field_name: value.#field_name.clone()
                });
                to_orig_fields.push(quote! {
                    #field_name: rkyv_val.#field_name.clone()
                });
            }
        } else if is_custom_type(field_type) {
            // Custom struct type - prepend R
            let rkyv_field_type = convert_custom_type(field_type);
            rkyv_fields.push(quote! {
                pub #field_name: #rkyv_field_type
            });
            from_orig_fields.push(quote! {
                #field_name: (&value.#field_name).into()
            });
            to_orig_fields.push(quote! {
                #field_name: (&rkyv_val.#field_name).into()
            });
        } else {
            // Primitive types - copy as is
            rkyv_fields.push(quote! {
                pub #field_name: #field_type
            });
            from_orig_fields.push(quote! {
                #field_name: value.#field_name.clone()
            });
            to_orig_fields.push(quote! {
                #field_name: rkyv_val.#field_name.clone()
            });
        }
    }

    let output = quote! {
        #[derive(::rkyv::Archive, ::rkyv::Serialize, ::rkyv::Deserialize, Clone, Debug, PartialEq)]
        #[rkyv(attr(doc = "rkyv-generated archived version"))]
        pub struct #rkyv_name {
            #(#rkyv_fields),*
        }

        impl From<&#orig_name> for #rkyv_name {
            fn from(value: &#orig_name) -> Self {
                Self {
                    #(#from_orig_fields),*
                }
            }
        }

        impl From<&#rkyv_name> for #orig_name {
            fn from(rkyv_val: &#rkyv_name) -> Self {
                Self {
                    #(#to_orig_fields),*
                }
            }
        }
    };

    output.into()
}

// Helper functions for type detection and conversion

fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Vec";
        }
    }
    false
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn is_custom_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.first() {
            let ident_str = segment.ident.to_string();
            // Check if it's not a primitive or std type
            // Only convert types ending in specific patterns to avoid enums
            return !matches!(
                ident_str.as_str(),
                "u8" | "u16" | "u32" | "u64" | "usize"
                | "i8" | "i16" | "i32" | "i64" | "isize"
                | "f32" | "f64" | "bool" | "String" | "str"
                | "Vec" | "Option" | "HashMap" | "BTreeMap" | "HashSet"
                | "DateTime" | "NaiveDateTime"
            ) && ident_str.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                && (ident_str.ends_with("Data")
                || ident_str.ends_with("State")
                || ident_str.ends_with("System")
                || ident_str.ends_with("Identification")
                || ident_str.ends_with("Value")
                || ident_str.ends_with("Altitudes")
                || ident_str.ends_with("Location")
                || ident_str.ends_with("Coordinates")
                || ident_str.ends_with("Log")
                || ident_str.ends_with("Vector")
                || ident_str.ends_with("Point")
                || ident_str.ends_with("Info")
                || ident_str.ends_with("Config"));
        }
    }
    false
}

fn needs_conversion(ty: &Type) -> bool {
    is_custom_type(ty) ||
        (if let Type::Path(tp) = ty {
            tp.path.segments.iter().any(|s| s.ident == "DateTime")
        } else {
            false
        })
}

fn extract_vec_inner(ty: &Type) -> Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return inner.clone();
                }
            }
        }
    }
    panic!("Cannot extract Vec inner type");
}

fn extract_option_inner(ty: &Type) -> Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return inner.clone();
                }
            }
        }
    }
    panic!("Cannot extract Option inner type");
}

fn convert_vec_type(ty: &Type) -> proc_macro2::TokenStream {
    let inner = extract_vec_inner(ty);
    if needs_conversion(&inner) {
        let rkyv_inner = convert_custom_type(&inner);
        quote! { Vec<#rkyv_inner> }
    } else {
        quote! { #ty }
    }
}

fn convert_option_type(ty: &Type) -> proc_macro2::TokenStream {
    let inner = extract_option_inner(ty);
    if needs_conversion(&inner) {
        let rkyv_inner = convert_custom_type(&inner);
        quote! { Option<#rkyv_inner> }
    } else {
        quote! { #ty }
    }
}

fn convert_custom_type(ty: &Type) -> proc_macro2::TokenStream {
    if let Type::Path(type_path) = ty {
        let mut new_path = type_path.clone();
        if let Some(segment) = new_path.path.segments.last_mut() {
            let old_ident = segment.ident.to_string();
            segment.ident = format_ident!("R{}", old_ident);
        }
        quote! { #new_path }
    } else {
        quote! { #ty }
    }
}
