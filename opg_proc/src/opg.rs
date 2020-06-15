use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::export::ToTokens;
use syn::spanned::Spanned;
use syn::Meta::*;
use syn::NestedMeta::*;

use crate::ast::*;
use crate::attr;
use crate::attr::ModelType;
use crate::fragment::*;
use crate::parsing_context::*;
use crate::symbol::*;

pub fn impl_derive_example(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, Vec<syn::Error>> {
    let cx = ParsingContext::new();
    let container = match Container::from_ast(&cx, &input) {
        Some(container) => container,
        None => return Err(cx.check().unwrap_err()),
    };
    cx.check()?;

    let ident = &container.ident;

    Ok(proc_macro2::TokenStream::new())
}

struct Parameters {
    self_var: syn::Ident,
    this: syn::Path,
}

impl Parameters {
    fn new(container: &Container) -> Self {
        let self_var = syn::Ident::new("self", Span::call_site());
        let this = container.ident.clone().into();

        Parameters { self_var, this }
    }
}

fn serialize_body(container: &Container, params: &Parameters) -> Fragment {
    if container.attrs.transparent {
        serialize_transparent(container, params)
    } else {
        match &container.data {
            Data::Enum(variants) => serialize_enum(params, variants, &container.attrs),
            Data::Struct(StructStyle::Struct, fields) => {
                serialize_struct(params, fields, &container.attrs)
            }
            Data::Struct(StructStyle::Tuple, fields) => {
                serialize_tuple_struct(params, fields, &container.attrs)
            }
            Data::Struct(StructStyle::NewType, fields) => {
                serialize_newtype_struct(params, &fields[0], &container.attrs)
            }
            _ => unimplemented!(),
        }
    }
}

fn serialize_transparent(container: &Container, params: &Parameters) -> Fragment {
    let fields = match &container.data {
        Data::Struct(_, fields) => fields,
        Data::Enum(_) => unreachable!(),
    };

    let self_var = &params.self_var;
    let transparent_field = fields.iter().find(|f| f.attrs.transparent).unwrap();
    let member = &transparent_field.member;

    let path = {
        let span = transparent_field.original.span();
        quote_spanned!(span=> )
    };

    Fragment::Block(quote! {
        #path()
    })
}

fn serialize_newtype_struct(
    params: &Parameters,
    field: &Field,
    attrs: &attr::Container,
) -> proc_macro2::TokenStream {
    let type_name = attrs.name.raw();

    let member_type_name: syn::Type = field.member_type.clone();

    let description = option_string(&attrs.description);
    let format = option_string(&attrs.format);
    let example = option_string(&attrs.example);

    let data = match attrs.model_type {
        ModelType::NewTypeString => quote_spanned! {span=>
            opg::ModelTypeDescription::String(opg::ModelString {
                variants: vec![],
                data: opg::ModelSimple {
                    format: #format,
                    example: #example,
                }
            })
        },
        ModelType::NewTypeInteger => quote_spanned! {span=>
            opg::ModelTypeDescription::Integer(opg::ModelSimple {
                format: #format,
                example: #example,
            })
        },
        ModelType::NewTypeNumber => quote_spanned! {span=>
            opg::ModelTypeDescription::Number(opg::ModelSimple {
                format: #format,
                example: #example,
            })
        },
        ModelType::NewTypeBoolean => quote_spanned! {span=>
            opg::ModelTypeDescription::Boolean
        },
        ModelType::NewTypeArray if attrs.inline => quote_spanned! {span=>
            opg::ModelTypeDescription::Array(opg::ModelArray {
                items: Box::new(opg::ModelReference::Inline(#member_type_name::get_structure()))
            })
        },
        ModelType::NewTypeArray => {
            let member_type_name = member_type_name
                .to_token_stream()
                .to_string()
                .replace(' ', "");
            let member_type_name = syn::LitStr::new(member_type_name.as_str(), Span::call_site());

            quote_spanned! {span=>
                opg::ModelTypeDescription::Array(opg::ModelArray {
                    items: Box::new(opg::ModelReference::Link(opg::ModelReferenceLink {
                        reference: #member_type_name,
                    }))
                })
            }
        }
        _ => unreachable!(),
    };

    quote_spanned! {span=>
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::Single(#data),
                }
            }
        }
    }
}

fn option_string(data: &Option<String>) -> proc_macro2::TokenStream {
    match data {
        Some(data) => {
            let string = syn::LitStr::new(data.as_str(), Span::call_site());
            quote! { Some(#string.to_owned()) }
        }
        None => quote! { None },
    }
}

fn get_member(params: &Parameters, _field: &Field, member: &syn::Member) -> TokenStream {
    let self_var = &params.self_var;
    quote! {
        &#self_var.#member
    }
}
