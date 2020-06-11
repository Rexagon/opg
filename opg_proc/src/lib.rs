extern crate proc_macro2;
extern crate quote;
extern crate syn;

mod example;
mod parsing_context;

use proc_macro::TokenStream;
use quote::quote;

use self::example::*;

#[proc_macro_derive(Example, attributes(example))]
pub fn derive_example(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    impl_derive_example(input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}
