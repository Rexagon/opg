use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::quote;
use syn::export::ToTokens;
use syn::Meta::*;
use syn::NestedMeta::*;

use crate::attr::*;
use crate::parsing_context::*;
use crate::symbol::*;

pub struct Name {
    name: String,
    renamed: bool,
}

impl Name {
    pub fn from_attrs(source_name: String, serialized_name: Attr<String>) -> Self {
        let serialized_name = serialized_name.get();
        let renamed = serialized_name.is_some();

        Self {
            name: serialized_name.unwrap_or_else(|| source_name.clone()),
            renamed,
        }
    }

    pub fn serialize(&self) -> String {
        self.name.clone()
    }
}
