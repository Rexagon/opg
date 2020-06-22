use proc_macro2::Span;
use quote::quote;

use crate::ast::*;
use crate::attr::{ModelType, TagType};
use crate::dummy;
use crate::parsing_context::*;

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

    let result = serialize_body(&container);

    //println!("{}", result.to_string());

    Ok(dummy::wrap_in_const("OPG_MODEL", ident, result))
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
    let description = option_string(&container.attrs.description);

    let variants = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .map(|variant| variant.attrs.name.serialized())
        .collect::<Vec<_>>();

    let example = option_string(&variants.first().cloned());

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(_opg::ModelTypeDescription::String(_opg::ModelString {
                variants: Some(vec![#(#variants.to_owned()),*]),
                data: _opg::ModelSimple {
                    format: None,
                    example: #example,
                }
            }))
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_untagged_enum(
    container: &Container,
    variants: &Vec<Variant>,
) -> proc_macro2::TokenStream {
    let description = option_string(&container.attrs.description);

    let one_of = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .map(|variant| match &variant.style {
            StructStyle::NewType => {
                let type_name = &variant.fields[0].original.ty;

                let inline = variant.attrs.inline;
                let description = option_string(&variant.attrs.description);

                quote! {
                    <#type_name as _opg::OpgModel>::select_reference(
                        #inline,
                        &_opg::ContextParams {
                            description: #description,
                            variants: None,
                            format: None,
                            example: None,
                        },
                    )
                }
            }
            StructStyle::Struct => {
                let description = option_string(&variant.attrs.description);

                let object_type_description = object_type_description(&variant.fields, |field| {
                    variant.attrs.inline || field.attrs.inline
                });

                quote! {
                    _opg::ModelReference::Inline(_opg::Model {
                        description: #description,
                        data: _opg::ModelData::Single(#object_type_description)
                    })
                }
            }
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::OneOf(_opg::ModelOneOf {
                one_of: vec![#(#one_of),*],
            })
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_adjacent_tagged_enum(
    container: &Container,
    variants: &Vec<Variant>,
    tag: &str,
    content: &str,
) -> proc_macro2::TokenStream {
    let description = option_string(&container.attrs.description);

    let (variants, one_of) = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .fold(
            (Vec::new(), Vec::new()),
            |(mut variants, mut one_of), variant| {
                let variant_name = variant.attrs.name.serialized();

                let type_description = match &variant.style {
                    StructStyle::NewType => field_model_reference(
                        &variant.attrs.description,
                        &variant.attrs.format,
                        &variant.attrs.example,
                        &variant.fields[0],
                        variant.attrs.inline,
                    ),
                    StructStyle::Tuple => tuple_model_reference(
                        &variant.attrs.description,
                        &variant.fields,
                        |field| variant.attrs.inline || field.attrs.inline,
                    ),
                    StructStyle::Struct => struct_model_reference(
                        &variant.attrs.description,
                        &variant.fields,
                        |field| field.attrs.inline || variant.attrs.inline,
                    ),
                    _ => unreachable!(),
                };

                variants.push(variant_name);
                one_of.push(type_description);
                (variants, one_of)
            },
        );

    let type_example = option_string(&variants.first().cloned());
    let type_name_stringified = container.ident.to_string();

    let struct_type_description = quote! {
        {
            let mut properties = std::collections::BTreeMap::new();
            let mut required = Vec::new();

            properties.insert(#tag.to_owned(), _opg::ModelReference::Inline(
                _opg::Model {
                    description: Some(format!("{} type variant", #type_name_stringified)),
                    data: _opg::ModelData::Single(_opg::ModelTypeDescription::String(_opg::ModelString {
                        variants: Some(vec![#(#variants.to_owned()),*]),
                        data: _opg::ModelSimple {
                            format: None,
                            example: #type_example,
                        }
                    }))
                }
            ));
            required.push(#tag.to_owned());

            properties.insert(#content.to_owned(), _opg::ModelReference::Inline(
                _opg::Model {
                    description: #description,
                    data: _opg::ModelData::OneOf(_opg::ModelOneOf {
                        one_of: vec![#(#one_of),*],
                    })
                }
            ));
            required.push(#content.to_owned());

            _opg::ModelTypeDescription::Object(
                _opg::ModelObject {
                    properties,
                    required,
                    ..Default::default()
                }
            )
        }
    };

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(#struct_type_description)
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_external_tagged_enum(
    container: &Container,
    variants: &Vec<Variant>,
) -> proc_macro2::TokenStream {
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
                            _opg::ModelReference::Inline(
                                _opg::Model {
                                    description: #description,
                                    data: _opg::ModelData::Single(_opg::ModelTypeDescription::String(_opg::ModelString {
                                        variants: Some(vec![#variant_name.to_owned()]),
                                        data: _opg::ModelSimple {
                                            format: None,
                                            example: #variant_name.to_owned(),
                                        }
                                    }))
                                }
                            )
                        }
                    }
                    StructStyle::NewType => field_model_reference(
                        &variant.attrs.description,
                        &variant.attrs.format,
                        &variant.attrs.example,
                        &variant.fields[0],
                        variant.attrs.inline,
                    ),
                    StructStyle::Tuple => {
                        tuple_model_reference(
                            &variant.attrs.description, &variant.fields, |field| {
                            variant.attrs.inline || field.attrs.inline
                        })
                    }
                    StructStyle::Struct => struct_model_reference(
                        &variant.attrs.description,
                        &variant.fields,
                        |field| {
                            variant.attrs.inline || field.attrs.inline
                        },
                    ),
                };

                variants.push(variant_name);
                one_of.push(type_description);
                (variants, one_of)
            },
        );

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(_opg::ModelTypeDescription::Object(
                _opg::ModelObject {
                    additional_properties: Some(Box::new(_opg::ModelReference::Inline(
                        _opg::Model {
                            description: #description,
                            data: _opg::ModelData::OneOf(_opg::ModelOneOf {
                                one_of: vec![#(#one_of),*],
                            })
                        }
                    ))),
                    ..Default::default()
                }
            ))
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_internal_tagged_enum(
    container: &Container,
    variants: &Vec<Variant>,
    tag: &str,
) -> proc_macro2::TokenStream {
    let description = option_string(&container.attrs.description);

    let type_name_stringified = container.ident.to_string();

    let one_of = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .map(|variant| {
            let variant_name = variant.attrs.name.serialized();
            let description = option_string(&variant.attrs.description);

            let model = match &variant.style {
                StructStyle::NewType => {
                    let type_name = &variant.fields[0].original.ty;

                    let format = option_string(&variant.fields[0].attrs.format);
                    let example = option_string(&variant.fields[0].attrs.example);

                    quote! {
                        <#type_name as _opg::OpgModel>::get_structure_with_params(&_opg::ContextParams {
                            description: #description,
                            variants: None,
                            format: #format,
                            example: #example,
                        })
                    }
                }
                StructStyle::Struct => {
                    let object_type_description =
                        object_type_description(&variant.fields, |field| {
                            variant.attrs.inline || field.attrs.inline
                        });

                    quote! {
                        _opg::Model {
                            description: #description,
                            data: _opg::ModelData::Single(#object_type_description)
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

                        properties.insert(#tag.to_owned(), _opg::ModelReference::Inline(
                            _opg::Model {
                                description: Some(format!("{} type variant", #type_name_stringified)),
                                data: _opg::ModelData::Single(_opg::ModelTypeDescription::String(_opg::ModelString {
                                    variants: Some(vec![#variant_name.to_owned()]),
                                    data: _opg::ModelSimple {
                                        format: None,
                                        example: Some(#variant_name.to_owned()),
                                    }
                                }))
                            }
                        ));

                        _opg::ModelTypeDescription::Object(_opg::ModelObject {
                            properties,
                            required: vec![#tag.to_owned()],
                            ..Default::default()
                        })
                    };

                    let _ = model.try_merge(_opg::Model {
                        description: None,
                        data: _opg::ModelData::Single(additional_object)
                    });

                    _opg::ModelReference::Inline(model)
                }
            }
        })
        .collect::<Vec<_>>();

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::OneOf(_opg::ModelOneOf {
                one_of: vec![#(#one_of),*],
            })
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_struct(container: &Container, fields: &Vec<Field>) -> proc_macro2::TokenStream {
    let description = option_string(&container.attrs.description);

    let object_type_description = object_type_description(fields, |field| field.attrs.inline);

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(#object_type_description)
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_tuple_struct(container: &Container, fields: &Vec<Field>) -> proc_macro2::TokenStream {
    let description = option_string(&container.attrs.description);

    let tuple_type_description = tuple_type_description(fields, |field| field.attrs.inline);

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(#tuple_type_description)
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_newtype_struct(container: &Container, field: &Field) -> proc_macro2::TokenStream {
    let description = option_string(&container.attrs.description);
    let format = option_string(&container.attrs.format);
    let example = option_string(&container.attrs.example);

    let type_name = &field.original.ty;

    let body = match container.attrs.explicit_model_type {
        Some(model_type) => newtype_model(description, format, example, model_type),
        None => {
            quote! {
                <#type_name as _opg::OpgModel>::get_structure_with_params(&_opg::ContextParams {
                    description: #description,
                    variants: None,
                    format: #format,
                    example: #example,
                })
            }
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
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
        _opg::ModelReference::Inline(_opg::Model {
            description: #description,
            data: _opg::ModelData::Single(#tuple_type_description)
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
        _opg::ModelReference::Inline(_opg::Model {
            description: #description,
            data: _opg::ModelData::Single(#object_type_description)
        })
    }
}

fn tuple_type_description<P>(fields: &Vec<Field>, inline_predicate: P) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let data = fields
        .iter()
        .map(|field| field_model_reference(&None, &None, &None, field, inline_predicate(field)))
        .collect::<Vec<_>>();

    let one_of = quote! {
        _opg::Model {
            description: None,
            data: _opg::ModelData::OneOf(_opg::ModelOneOf {
                one_of: vec![#(#data),*],
            })
        }
    };

    quote! {
        _opg::ModelTypeDescription::Array(
            _opg::ModelArray {
                items: Box::new(_opg::ModelReference::Inline(#one_of)),
            }
        )
    }
}

fn object_type_description<P>(fields: &Vec<Field>, inline_predicate: P) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let data = fields
        .iter()
        .filter(|field| !field.attrs.skip_serializing)
        .map(|field| {
            let field_model_reference = field_model_reference(
                &field.attrs.description,
                &field.attrs.format,
                &field.attrs.example,
                field,
                inline_predicate(field),
            );

            let property_name = syn::LitStr::new(&field.attrs.name.serialized(), Span::call_site());

            let push_required = if !field.attrs.optional {
                quote!( required.push(#property_name.to_owned()) )
            } else {
                quote!()
            };

            quote! {
                properties.insert(#property_name.to_owned(), #field_model_reference);
                #push_required;
            }
        })
        .collect::<Vec<_>>();

    quote! {
        {
            let mut properties = std::collections::BTreeMap::new();
            let mut required = Vec::new();

            #(#data)*

            _opg::ModelTypeDescription::Object(
                _opg::ModelObject {
                    properties,
                    required,
                    ..Default::default()
                }
            )
        }
    }
}

fn field_model_reference(
    description: &Option<String>,
    format: &Option<String>,
    example: &Option<String>,
    field: &Field,
    inline: bool,
) -> proc_macro2::TokenStream {
    let type_name = &field.original.ty;

    let description = option_string(description);
    let format = option_string(format);
    let example = option_string(example);

    match field.attrs.explicit_model_type {
        Some(model_type) if model_type.is_newtype() => {
            let model = newtype_model(description, format, example, model_type);

            quote! {
                _opg::ModelReference::Inline(#model)
            }
        }
        _ => {
            quote! {
                <#type_name as _opg::OpgModel>::select_reference(
                    #inline,
                    &_opg::ContextParams {
                        description: #description,
                        variants: None,
                        format: #format,
                        example: #example,
                    },
                )
            }
        }
    }
}

fn newtype_model(
    description: proc_macro2::TokenStream,
    format: proc_macro2::TokenStream,
    example: proc_macro2::TokenStream,
    model_type: ModelType,
) -> proc_macro2::TokenStream {
    let data = match model_type {
        ModelType::NewTypeString => quote! {
            _opg::ModelTypeDescription::String(_opg::ModelString {
                variants: None,
                data: _opg::ModelSimple {
                    format: #format,
                    example: #example,
                }
            })
        },
        ModelType::NewTypeInteger => quote! {
            _opg::ModelTypeDescription::Integer(_opg::ModelSimple {
                format: #format,
                example: #example,
            })
        },
        ModelType::NewTypeNumber => quote! {
            _opg::ModelTypeDescription::Number(_opg::ModelSimple {
                format: #format,
                example: #example,
            })
        },
        ModelType::NewTypeBoolean => quote! {
            _opg::ModelTypeDescription::Boolean
        },
        ModelType::NewTypeArray => todo!(),
        _ => unreachable!(),
    };

    quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(#data),
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

fn implement_type(
    type_name: &syn::Ident,
    body: proc_macro2::TokenStream,
    inline: bool,
) -> proc_macro2::TokenStream {
    let inline = if inline {
        quote! {
            #[inline(always)]
            fn select_reference(_: bool, inline_params: &_opg::ContextParams) -> _opg::ModelReference {
                Self::inject(_opg::InjectReference::Inline(inline_params))
            }
        }
    } else {
        quote! {
            #[inline(always)]
            fn get_type_name() -> Option<&'static str> {
                Some(stringify!(#type_name))
            }
        }
    };

    quote! {
        impl _opg::OpgModel for #type_name {
            fn get_structure() -> _opg::Model {
                #body
            }

            #inline
        }
    }
}
