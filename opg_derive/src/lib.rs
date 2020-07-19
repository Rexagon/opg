extern crate proc_macro2;
extern crate quote;
extern crate syn;

mod ast;
mod attr;
mod bound;
mod case;
mod dummy;
mod opg;
mod parsing_context;
mod symbol;

use proc_macro::TokenStream;
use quote::quote;

use self::opg::*;

#[proc_macro_derive(OpgModel, attributes(opg))]
pub fn derive_opg_model(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    impl_derive_opg_model(input)
        .unwrap_or_else(to_compile_errors)
        .into()
}

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}
