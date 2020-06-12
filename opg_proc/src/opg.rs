use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::quote;
use syn::export::ToTokens;
use syn::Meta::*;
use syn::NestedMeta::*;

use crate::parsing_context::*;
use crate::symbol::*;

pub fn impl_derive_example(
    ast: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, Vec<syn::Error>> {
    let name = &ast.ident;

    Ok(proc_macro2::TokenStream::new())
}

#[derive(Default)]
struct StructOrEnumParams {
    example: Option<String>,
    with: Option<syn::ExprPath>,
}
