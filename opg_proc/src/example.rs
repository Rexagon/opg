use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::quote;
use syn::Meta::*;
use syn::NestedMeta::*;

use crate::parsing_context::*;

pub fn impl_derive_example(
    ast: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, Vec<syn::Error>> {
    let name = &ast.ident;

    let mut params = ExampleDeriveParams::default();

    let cx = ParsingContext::new();

    for arg in ast
        .attrs
        .iter()
        .flat_map(|attr| get_meta_items(&cx, attr))
        .flatten()
    {
        match arg {
            Meta(NameValue(m)) if m.path.is_ident("with") => {
                if let Ok(with) = parse_lit_into_expr_path(&cx, "with", &m.lit) {
                    params.with = Some(with);
                }
            }
            Lit(syn::Lit::Str(value)) => params.example = value.parse::<syn::Ident>().ok(),
            other => cx.error_spanned_by(other, "unknown attribute parameter"),
        }
    }

    cx.check()?;

    let operation = if let Some(example) = params.example {
        quote! {
            #example.to_owned()
        }
    } else if let Some(with) = params.with {
        quote! { Some(#with()) }
    } else {
        quote! { None }
    };

    Ok(quote! {
        impl Example for #name {
            fn example() -> Option<String> {
                #operation
            }
        }
    })
}

#[derive(Default)]
struct ExampleDeriveParams {
    example: Option<syn::Ident>,
    with: Option<syn::Ident>,
}

fn get_meta_items(cx: &ParsingContext, attr: &syn::Attribute) -> Result<Vec<syn::NestedMeta>, ()> {
    if !attr.path.is_ident(ATTRIBUTE_NAME) {
        println!("aaa");
        return Ok(Vec::new());
    }

    match attr.parse_meta() {
        Ok(List(meta)) => Ok(meta.nested.into_iter().collect()),
        Ok(other) => {
            cx.error_spanned_by(other, "expected #[example(...)]");
            Err(())
        }
        Err(err) => {
            cx.syn_error(err);
            Err(())
        }
    }
}

fn parse_lit_into_expr_path(
    cx: &ParsingContext,
    attr_name: &'static str,
    lit: &syn::Lit,
) -> Result<syn::Ident, ()> {
    let string = get_lit_str(cx, attr_name, lit)?;
    parse_lit_str(string).map_err(|_| {
        cx.error_spanned_by(
            lit,
            format!("failed to parse path expr: {:?}", string.value()),
        )
    })
}

fn parse_lit_str<T>(s: &syn::LitStr) -> syn::parse::Result<T>
where
    T: syn::parse::Parse,
{
    let tokens = spanned_tokens(s)?;
    syn::parse2(tokens)
}

fn spanned_tokens(s: &syn::LitStr) -> syn::parse::Result<TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan_token_stream(stream, s.span()))
}

fn respan_token_stream(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|token| respan_token_tree(token, span))
        .collect()
}

fn respan_token_tree(mut token: TokenTree, span: Span) -> TokenTree {
    if let TokenTree::Group(g) = &mut token {
        *g = Group::new(g.delimiter(), respan_token_stream(g.stream(), span));
    }
    token.set_span(span);
    token
}

fn get_lit_str<'a>(
    cx: &ParsingContext,
    attr_name: &'static str,
    lit: &'a syn::Lit,
) -> Result<&'a syn::LitStr, ()> {
    if let syn::Lit::Str(lit) = lit {
        Ok(lit)
    } else {
        cx.error_spanned_by(
            lit,
            format!(
                "expected example {} attribute to be a string: `{} = \"...\"`",
                attr_name, attr_name
            ),
        );
        Err(())
    }
}

const ATTRIBUTE_NAME: &'static str = "example";
