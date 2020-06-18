use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{quote, quote_spanned};
use syn::export::ToTokens;
use syn::spanned::Spanned;
use syn::Meta::*;
use syn::NestedMeta::*;

use crate::ast::*;
use crate::attr;
use crate::attr::{ModelType, TagType};
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
        Data::Enum(variants) => serialize_enum(container, variants),
        Data::Struct(StructStyle::Struct, fields) => serialize_struct(container, fields),
        Data::Struct(StructStyle::Tuple, fields) => serialize_tuple_struct(container, fields),
        Data::Struct(StructStyle::NewType, fields) => {
            serialize_newtype_struct(container, &fields[0])
        }
        _ => unimplemented!(),
    }
}

fn serialize_enum(container: &Container, variants: &Vec<Variant>) -> proc_macro2::TokenStream {
    return match (container.attrs.model_type, &container.attrs.tag_type) {
        (ModelType::NewTypeString, _) => serialize_newtype_enum(container, variants),
        (ModelType::Object, TagType::Adjacent { tag, content }) => {
            serialize_adjacent_tagged_enum(container, variants, tag, content)
        }
        (ModelType::Dictionary, _) => serialize_external_tagged_enum(container, variants),
        (ModelType::OneOf, TagType::None) => serialize_untagged_enum(container, variants),
        (ModelType::OneOf, TagType::Internal { tag }) => {
            serialize_internal_tagged_enum(container, variants, tag)
        }
        _ => unreachable!(),
    };
}

fn serialize_newtype_enum(
    container: &Container,
    variants: &Vec<Variant>,
) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let variants = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .map(|variant| variant.attrs.name.serialized())
        .collect::<Vec<_>>();

    let example = option_string(&variants.first().cloned());

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::Single(opg::ModelTypeDescription::String(opg::ModelString {
                        variants: Some(vec![#(#variants.to_owned()),*]),
                        data: opg::ModelSimple {
                            format: None,
                            example: #example,
                        }
                    }))
                }
            }
        }
    }
}

fn serialize_untagged_enum(
    container: &Container,
    variants: &Vec<Variant>,
) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let one_of = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .map(|variant| match &variant.style {
            StructStyle::NewType => {
                let description = option_string(&variant.attrs.description);

                if variant.attrs.inline {
                    let member_type_name = &variant.fields[0].original.ty;

                    quote! {
                        opg::ModelReference::Inline(<#member_type_name>::get_structure_with_params(&opg::ContextParams {
                            description: #description,
                            variants: None,
                            format: None,
                            example: None,
                        }))
                    }
                } else {
                    let member_type_name = stringify_type(&variant.fields[0].original.ty);

                    quote! {
                        opg::ModelReference::Link(opg::ModelReferenceLink {
                            reference: #member_type_name.to_owned(),
                        })
                    }
                }
            }
            StructStyle::Struct => {
                let description = option_string(&variant.attrs.description);

                let object_type_description = object_type_description(&variant.fields, |field| field.attrs.inline || variant.attrs.inline || container.attrs.inline);

                quote! {
                    opg::ModelReference::Inline(opg::Model {
                        description: #description,
                        data: opg::ModelData::Single(#object_type_description)
                    })
                }
            }
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::OneOf(opg::ModelOneOf {
                        one_of: vec![#(#one_of),*],
                    })
                }
            }
        }
    }
}

fn serialize_adjacent_tagged_enum(
    container: &Container,
    variants: &Vec<Variant>,
    tag: &str,
    content: &str,
) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let (variants, one_of) = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .fold(
            (Vec::new(), Vec::new()),
            |(mut variants, mut one_of), variant| {
                let variant_name = variant.attrs.name.serialized();

                let type_description = match &variant.style {
                    StructStyle::NewType => newtype_model_reference(
                        &variant.attrs.description,
                        &variant.fields[0],
                        variant.attrs.inline,
                    ),
                    StructStyle::Tuple => {
                        tuple_model_reference(&variant.attrs.description, &variant.fields, |_| {
                            container.attrs.inline || variant.attrs.inline
                        })
                    }
                    StructStyle::Struct => struct_model_reference(
                        &variant.attrs.description,
                        &variant.fields,
                        |field| {
                            field.attrs.inline || variant.attrs.inline || container.attrs.inline
                        },
                    ),
                    _ => unreachable!(),
                };

                variants.push(variant_name);
                one_of.push(type_description);
                (variants, one_of)
            },
        );

    let type_example = option_string(&variants.first().cloned());
    let type_name_stringified = type_name.to_string();

    let struct_type_description = quote! {
        {
            let mut properties = std::collections::BTreeMap::new();
            let mut required = Vec::new();

            properties.insert(#tag.to_owned(), opg::ModelReference::Inline(
                opg::Model {
                    description: Some(format!("{} type variant", #type_name_stringified)),
                    data: opg::ModelData::Single(opg::ModelTypeDescription::String(opg::ModelString {
                        variants: Some(vec![#(#variants.to_owned()),*]),
                        data: opg::ModelSimple {
                            format: None,
                            example: #type_example,
                        }
                    }))
                }
            ));
            required.push(#tag.to_owned());

            properties.insert(#content.to_owned(), opg::ModelReference::Inline(
                opg::Model {
                    description: #description,
                    data: opg::ModelData::OneOf(opg::ModelOneOf {
                        one_of: vec![#(#one_of),*],
                    })
                }
            ));
            required.push(#content.to_owned());

            opg::ModelTypeDescription::Object(
                opg::ModelObject {
                    properties,
                    required,
                    ..Default::default()
                }
            )
        }
    };

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::Single(#struct_type_description)
                }
            }
        }
    }
}

fn serialize_external_tagged_enum(
    container: &Container,
    variants: &Vec<Variant>,
) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let (_, one_of) = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .fold(
            (Vec::new(), Vec::new()),
            |(mut variants, mut one_of), variant| {
                let variant_name = variant.attrs.name.serialized();

                let type_description = match &variant.style {
                    StructStyle::Unit => {
                        let description = option_string(&variant.attrs.description);

                        quote! {
                            opg::ModelReference::Inline(
                                opg::Model {
                                    description: #description,
                                    data: opg::ModelData::Single(opg::ModelTypeDescription::String(opg::ModelString {
                                        variants: Some(vec![#variant_name.to_owned()]),
                                        data: opg::ModelSimple {
                                            format: None,
                                            example: #variant_name.to_owned(),
                                        }
                                    }))
                                }
                            )
                        }
                    }
                    StructStyle::NewType => newtype_model_reference(
                        &variant.attrs.description,
                        &variant.fields[0],
                        variant.attrs.inline,
                    ),
                    StructStyle::Tuple => {
                        tuple_model_reference(&variant.attrs.description, &variant.fields, |_| {
                            container.attrs.inline || variant.attrs.inline
                        })
                    }
                    StructStyle::Struct => struct_model_reference(
                        &variant.attrs.description,
                        &variant.fields,
                        |field| {
                            field.attrs.inline || variant.attrs.inline || container.attrs.inline
                        },
                    ),
                    _ => unreachable!(),
                };

                variants.push(variant_name);
                one_of.push(type_description);
                (variants, one_of)
            },
        );

    let struct_type_description = quote! {
        {
            opg::ModelTypeDescription::Object(
                opg::ModelObject {
                    additional_properties: Some(Box::new(opg::ModelReference::Inline(
                        opg::Model {
                            description: #description,
                            data: opg::ModelData::OneOf(opg::ModelOneOf {
                                one_of: vec![#(#one_of),*],
                            })
                        }
                    ))),
                    ..Default::default()
                }
            )
        }
    };

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::Single(#struct_type_description)
                }
            }
        }
    }
}

fn serialize_internal_tagged_enum(
    container: &Container,
    variants: &Vec<Variant>,
    tag: &str,
) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let type_name_stringified = type_name.to_string();

    let one_of = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .map(|variant| {
            let variant_name = variant.attrs.name.serialized();
            let description = option_string(&variant.attrs.description);

            let model = match &variant.style {
                StructStyle::NewType => {
                    let member_type_name = &variant.fields[0].original.ty;

                    quote! {
                        <#member_type_name>::get_structure_with_params(&opg::ContextParams {
                            description: #description,
                            variants: None,
                            format: None,
                            example: None,
                        })
                    }
                }
                StructStyle::Struct => {
                    let object_type_description =
                        object_type_description(&variant.fields, |field| {
                            field.attrs.inline || variant.attrs.inline || container.attrs.inline
                        });

                    quote! {
                        opg::Model {
                            description: #description,
                            data: opg::ModelData::Single(#object_type_description)
                        }
                    }
                }
                _ => unreachable!(),
            };

            quote! {
                {
                    let mut model = #model;

                    let additional_object = {
                        let mut properties = std::collections::BTreeMap::new();

                        properties.insert(#tag.to_owned(), opg::ModelReference::Inline(
                            opg::Model {
                                description: Some(format!("{} type variant", #type_name_stringified)),
                                data: opg::ModelData::Single(opg::ModelTypeDescription::String(opg::ModelString {
                                    variants: Some(vec![#variant_name.to_owned()]),
                                    data: opg::ModelSimple {
                                        format: None,
                                        example: Some(#variant_name.to_owned()),
                                    }
                                }))
                            }
                        ));

                        opg::ModelTypeDescription::Object(opg::ModelObject {
                            properties,
                            required: vec![#tag.to_owned()],
                            ..Default::default()
                        })
                    };

                    let _ = model.try_merge(opg::Model {
                        description: None,
                        data: opg::ModelData::Single(additional_object)
                    });

                    opg::ModelReference::Inline(model)
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::OneOf(opg::ModelOneOf {
                        one_of: vec![#(#one_of),*],
                    })
                }
            }
        }
    }
}

fn serialize_struct(container: &Container, fields: &Vec<Field>) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let object_type_description =
        object_type_description(fields, |field| container.attrs.inline || field.attrs.inline);

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::Single(#object_type_description)
                }
            }
        }
    }
}

fn serialize_tuple_struct(container: &Container, fields: &Vec<Field>) -> proc_macro2::TokenStream {
    let type_name = &container.ident;

    let description = option_string(&container.attrs.description);

    let tuple_type_description = tuple_type_description(fields, |_| container.attrs.inline);

    quote! {
        impl opg::OpgModel for #type_name {
            fn get_structure() -> opg::Model {
                opg::Model {
                    description: #description,
                    data: opg::ModelData::Single(#tuple_type_description)
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

fn newtype_model_reference(
    description: &Option<String>,
    field: &Field,
    inline: bool,
) -> proc_macro2::TokenStream {
    if inline {
        let member_type_name = &field.original.ty;

        let description = option_string(description);

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
    }
}

fn tuple_model_reference<P>(
    description: &Option<String>,
    fields: &Vec<Field>,
    inline_predicate: P,
) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let description = option_string(description);
    let tuple_type_description = tuple_type_description(fields, inline_predicate);

    quote! {
        opg::ModelReference::Inline(opg::Model {
            description: #description,
            data: opg::ModelData::Single(#tuple_type_description)
        })
    }
}

fn struct_model_reference<P>(
    description: &Option<String>,
    fields: &Vec<Field>,
    inline_predicate: P,
) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let description = option_string(description);
    let object_type_description = object_type_description(fields, inline_predicate);

    quote! {
        opg::ModelReference::Inline(opg::Model {
            description: #description,
            data: opg::ModelData::Single(#object_type_description)
        })
    }
}

fn tuple_type_description<P>(fields: &Vec<Field>, inline_predicate: P) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let data = fields
        .iter()
        .map(|field| {
            if inline_predicate(field) {
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
        opg::ModelTypeDescription::Array(
            opg::ModelArray {
                items: Box::new(opg::ModelReference::Inline(#one_of)),
            }
        )
    }
}

fn object_type_description<P>(fields: &Vec<Field>, inline_predicate: P) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let data = fields.iter().filter(|field| !field.attrs.skip_serializing).map(|field| {
        let field_model = if inline_predicate(field) {
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
        {
            let mut properties = std::collections::BTreeMap::new();
            let mut required = Vec::new();

            #(#data)*

            opg::ModelTypeDescription::Object(
                opg::ModelObject {
                    properties,
                    required,
                    ..Default::default()
                }
            )
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
