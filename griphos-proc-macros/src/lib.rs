use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(TypeStrOf)]
pub fn derive_has_type_str(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    quote! {
        impl TypeStrOf for #name {
            fn type_str_of() -> String { #name_str.to_string() }
        }
    }
    .into()
}
