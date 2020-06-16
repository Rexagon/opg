use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::export::ToTokens;
use syn::spanned::Spanned;
use syn::Meta::*;
use syn::NestedMeta::*;

use crate::ast::*;
use crate::attr;
use crate::attr::ModelType;
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

    let result = serialize_body(&container);

    println!("{}", result.to_string());

    Ok(result)
}

fn serialize_body(container: &Container) -> proc_macro2::TokenStream {
    match &container.data {
        //Data::Enum(variants) => serialize_enum(container, variants),
        Data::Struct(StructStyle::Struct, fields) => serialize_struct(container, fields),
        Data::Struct(StructStyle::Tuple, fields) => serialize_tuple_struct(container, fields),
        Data::Struct(StructStyle::NewType, fields) => {
            serialize_newtype_struct(container, &fields[0])
        }
        _ => unimplemented!(),
    }
}

fn serialize_struct(container: &Container, fields: &Vec<Field>) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let data = fields.iter().map(|field| {
        let field_model = if container.attrs.inline || field.attrs.inline {
            let member_type_name = &field.original.ty;

            let description = option_string(&field.attrs.description);
            // TODO: add variants attr
            // TODO: add format attr
            // TODO: add example attr

            quote! {
                opg::ModelReference::Inline(<#member_type_name>::get_structure_with_params(&opg::ContextParams {
                    description: #description,
                    variants: None,
                    format: None,
                    example: None,
                }))
            }
        } else {
            let member_type_name = stringify_type(&field.original.ty);

            quote! {
                opg::ModelReference::Link(opg::ModelReferenceLink {
                    reference: #member_type_name.to_owned(),
                })
            }
        };

        let property_name = syn::LitStr::new(&field.attrs.name.serialized(), Span::call_site());

        let push_required = if !field.attrs.optional {
            quote!( required.push(#property_name.to_owned()) )
        } else {
            quote!()
        };

        quote!{
            properties.insert(#property_name.to_owned(), #field_model);
            #push_required;
        }
    }).collect::<Vec<_>>();

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                let mut properties = std::collections::BTreeMap::new();
                let mut required = Vec::new();

                #(#data)*

                opg::Model {
                    description: #description,
                    data: opg::ModelData::Single(opg::ModelTypeDescription::Object(
                        opg::ModelObject {
                            properties,
                            required,
                        }
                    ))
                }
            }
        }
    }
}

fn serialize_tuple_struct(container: &Container, fields: &Vec<Field>) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let data = fields
        .iter()
        .map(|field| {
            if container.attrs.inline {
                let member_type_name = &field.original.ty;

                quote! {
                    opg::ModelReference::Inline(<#member_type_name>::get_structure())
                }
            } else {
                let member_type_name = stringify_type(&field.original.ty);

                quote! {
                    opg::ModelReference::Link(opg::ModelReferenceLink {
                        reference: #member_type_name.to_owned(),
                    })
                }
            }
        })
        .collect::<Vec<_>>();

    let one_of = quote! {
        opg::Model {
            description: None,
            data: opg::ModelData::OneOf(opg::ModelOneOf {
                one_of: vec![#(#data),*],
            })
        }
    };

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::Single(opg::ModelTypeDescription::Array(
                        opg::ModelArray {
                            items: Box::new(opg::ModelReference::Inline(#one_of)),
                        }
                    ))
                }
            }
        }
    }
}

fn serialize_newtype_struct(container: &Container, field: &Field) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);
    let format = option_string(&container.attrs.format);
    let example = option_string(&container.attrs.example);

    let member_type_name = &field.original.ty;

    let data = match container.attrs.model_type {
        ModelType::NewTypeString => quote! {
            opg::ModelTypeDescription::String(opg::ModelString {
                variants: None,
                data: opg::ModelSimple {
                    format: #format,
                    example: #example,
                }
            })
        },
        ModelType::NewTypeInteger => quote! {
            opg::ModelTypeDescription::Integer(opg::ModelSimple {
                format: #format,
                example: #example,
            })
        },
        ModelType::NewTypeNumber => quote! {
            opg::ModelTypeDescription::Number(opg::ModelSimple {
                format: #format,
                example: #example,
            })
        },
        ModelType::NewTypeBoolean => quote! {
            opg::ModelTypeDescription::Boolean
        },
        ModelType::NewTypeArray if container.attrs.inline => quote! {
            opg::ModelTypeDescription::Array(opg::ModelArray {
                items: Box::new(opg::ModelReference::Inline(<#member_type_name>::get_structure()))
            })
        },
        ModelType::NewTypeArray => {
            let member_type_name = stringify_type(member_type_name);

            quote! {
                opg::ModelTypeDescription::Array(opg::ModelArray {
                    items: Box::new(opg::ModelReference::Link(opg::ModelReferenceLink {
                        reference: #member_type_name.to_owned(),
                    }))
                })
            }
        }
        _ => unreachable!(),
    };

    quote! {
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

fn stringify_type(ty: &syn::Type) -> syn::LitStr {
    let name = ty.to_token_stream().to_string().replace(' ', "");
    syn::LitStr::new(name.as_str(), Span::call_site())
}
