use proc_macro2::Span;
use quote::quote;

use crate::ast::*;
use crate::attr::{self, ExplicitModelType, ModelType, TagType};
use crate::bound;
use crate::dummy;
use crate::parsing_context::*;

pub fn impl_derive_opg_model(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, Vec<syn::Error>> {
    let cx = ParsingContext::new();
    let container = match Container::from_ast(&cx, &input) {
        Some(container) => container,
        None => return Err(cx.check().unwrap_err()),
    };
    cx.check()?;

    let ident = &container.ident;
    let generics = build_generics(&container);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let body = serialize_body(&container);

    let result = quote! {
        impl #impl_generics _opg::OpgModel for #ident #ty_generics #where_clause {
            #body
        }
    };

    // println!("{}", result.to_string());

    Ok(dummy::wrap_in_const("OPG_MODEL", ident, result))
}

fn build_generics(cont: &Container) -> syn::Generics {
    let generics = bound::without_default(cont.generics);

    bound::with_bound(
        cont,
        &generics,
        |field, variant| {
            !field.skip_serializing && variant.map_or(true, |variant| !variant.skip_serializing)
        },
        &syn::parse_quote!(_opg::OpgModel),
    )
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

fn serialize_enum(container: &Container, variants: &[Variant]) -> proc_macro2::TokenStream {
    match (container.attrs.model_type, &container.attrs.tag_type) {
        (ModelType::NewType, _) => serialize_newtype_enum(container, variants),
        (ModelType::Object, TagType::Adjacent { tag, content }) => {
            serialize_adjacent_tagged_enum(container, variants, tag, content)
        }
        (ModelType::Dictionary, _) => serialize_external_tagged_enum(container, variants),
        (ModelType::OneOf, TagType::None) => serialize_untagged_enum(container, variants),
        (ModelType::OneOf, TagType::Internal { tag }) => {
            serialize_internal_tagged_enum(container, variants, tag)
        }
        _ => unreachable!(),
    }
}

fn serialize_newtype_enum(container: &Container, variants: &[Variant]) -> proc_macro2::TokenStream {
    let description = option_string(container.attrs.description.as_deref());

    let body = if container.attrs.has_repr {
        let variants = variants
            .iter()
            .filter(|variant| !variant.attrs.skip_serializing)
            .filter_map(|variant| {
                variant
                    .original
                    .discriminant
                    .as_ref()
                    .map(|(_, discriminant)| (variant.attrs.name.serialized(), discriminant))
            })
            .map(|(name, discriminant)| {
                let description = format!("{} variant", name);
                let example = quote::ToTokens::to_token_stream(discriminant).to_string();

                quote! {
                    _opg::Model {
                        description: Some(#description.to_owned()),
                        data: _opg::ModelData::Single(_opg::ModelType {
                            nullable: false,
                            type_description: _opg::ModelTypeDescription::Integer(_opg::ModelSimple {
                                format: None,
                                example: Some(#example.to_owned()),
                            })
                        }),
                    }
                }
            })
            .map(inline_reference)
            .collect::<Vec<_>>();

        quote! {
            _opg::Model {
                description: #description,
                data: _opg::ModelData::OneOf(_opg::ModelOneOf {
                    one_of: vec![#(#variants),*],
                })
            }
        }
    } else {
        let variants = variants
            .iter()
            .filter(|variant| !variant.attrs.skip_serializing)
            .map(|variant| variant.attrs.name.serialized())
            .collect::<Vec<_>>();

        let example = option_string(variants.first().map(|x| x.as_str()));

        quote! {
            _opg::Model {
                description: #description,
                data: _opg::ModelData::Single(_opg::ModelType {
                    nullable: false,
                    type_description: _opg::ModelTypeDescription::String(_opg::ModelString {
                        variants: Some(vec![#(#variants.to_owned()),*]),
                        data: _opg::ModelSimple {
                            format: None,
                            example: #example,
                        }
                    })
                })
            }
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_untagged_enum(
    container: &Container,
    variants: &[Variant],
) -> proc_macro2::TokenStream {
    let description = option_string(container.attrs.description.as_deref());

    let one_of = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .map(|variant| match &variant.style {
            StructStyle::NewType => {
                let field = &variant.fields[0];
                let context_params = ContextParams::from(&field.attrs).or(&variant.attrs);

                field_model_reference(context_params, field, variant.attrs.inline)
            }
            StructStyle::Struct => inline_reference(object_model(
                false,
                &variant.attrs.description,
                &variant.fields,
                |field| variant.attrs.inline || field.attrs.inline,
            )),
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
    variants: &[Variant],
    tag: &str,
    content: &str,
) -> proc_macro2::TokenStream {
    let description = option_string(container.attrs.description.as_deref());
    let nullable = container.attrs.nullable;

    let (variants, one_of) = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .fold(
            (Vec::new(), Vec::new()),
            |(mut variants, mut one_of), variant| {
                let variant_name = variant.attrs.name.serialized();

                let type_description = match &variant.style {
                    StructStyle::NewType => {
                        let field = &variant.fields[0];
                        let context_params = ContextParams::from(&field.attrs).or(&variant.attrs);

                        field_model_reference(context_params, field, variant.attrs.inline)
                    }
                    StructStyle::Tuple => inline_reference(tuple_model(
                        false,
                        &variant.attrs.description,
                        &variant.fields,
                        |field| variant.attrs.inline || field.attrs.inline,
                    )),
                    StructStyle::Struct => inline_reference(tuple_model(
                        false,
                        &variant.attrs.description,
                        &variant.fields,
                        |field| field.attrs.inline || variant.attrs.inline,
                    )),
                    _ => unreachable!(),
                };

                variants.push(variant_name);
                one_of.push(type_description);
                (variants, one_of)
            },
        );

    let type_example = option_string(variants.first().map(|x| x.as_str()));
    let type_name_stringified = container.ident.to_string();

    let struct_type_description = quote! {
        {
            let mut properties = std::collections::BTreeMap::new();
            let mut required = Vec::new();

            properties.insert(#tag.to_owned(), _opg::ModelReference::Inline(
                _opg::Model {
                    description: Some(format!("{} type variant", #type_name_stringified)),
                    data: _opg::ModelData::Single(_opg::ModelType {
                        nullable: false,
                        type_description: _opg::ModelTypeDescription::String(_opg::ModelString {
                            variants: Some(vec![#(#variants.to_owned()),*]),
                            data: _opg::ModelSimple {
                                format: None,
                                example: #type_example,
                            }
                        })
                    })
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
            data: _opg::ModelData::Single(_opg::ModelType {
                nullable: #nullable,
                type_description: #struct_type_description
            })
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_external_tagged_enum(
    container: &Container,
    variants: &[Variant],
) -> proc_macro2::TokenStream {
    let description = option_string(container.attrs.description.as_deref());
    let nullable = container.attrs.nullable;

    let (_, one_of) = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .fold(
            (Vec::new(), Vec::new()),
            |(mut variants, mut one_of), variant| {
                let variant_name = variant.attrs.name.serialized();

                let type_description = match &variant.style {
                    StructStyle::Unit => {
                        let description = option_string(variant.attrs.description.as_deref());

                        quote! {
                            _opg::ModelReference::Inline(
                                _opg::Model {
                                    description: #description,
                                    data: _opg::ModelData::Single(_opg::ModelType {
                                        nullable: false,
                                        type_description: _opg::ModelTypeDescription::String(_opg::ModelString {
                                            variants: Some(vec![#variant_name.to_owned()]),
                                            data: _opg::ModelSimple {
                                                format: None,
                                                example: Some(#variant_name.to_owned()),
                                            }
                                        })
                                    })
                                }
                            )
                        }
                    }
                    StructStyle::NewType => {
                        let field = &variant.fields[0];
                        let context_params = ContextParams::from(&field.attrs).or(&variant.attrs);

                        field_model_reference(
                            context_params,
                            field,
                            variant.attrs.inline,
                        )
                    },
                    StructStyle::Tuple => {
                        inline_reference(tuple_model(false,
                            &variant.attrs.description, &variant.fields, |field| {
                            variant.attrs.inline || field.attrs.inline
                        }))
                    }
                    StructStyle::Struct => inline_reference(object_model(false,
                        &variant.attrs.description,
                        &variant.fields,
                        |field| {
                            variant.attrs.inline || field.attrs.inline
                        },
                    )),
                };

                variants.push(variant_name);
                one_of.push(type_description);
                (variants, one_of)
            },
        );

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(_opg::ModelType {
                nullable: #nullable,
                type_description: _opg::ModelTypeDescription::Object(
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
                )
            })
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_internal_tagged_enum(
    container: &Container,
    variants: &[Variant],
    tag: &str,
) -> proc_macro2::TokenStream {
    let description = option_string(container.attrs.description.as_deref());
    let nullable = container.attrs.nullable;

    let type_name_stringified = container.ident.to_string();

    let one_of = variants
        .iter()
        .filter(|variant| !variant.attrs.skip_serializing)
        .map(|variant| {
            let variant_name = variant.attrs.name.serialized();

            let model = match &variant.style {
                StructStyle::Unit => {
                    object_model(false, &variant.attrs.description, &[], |_| false)
                }
                StructStyle::NewType => {
                    let field = &variant.fields[0];
                    let type_name = &field.original.ty;
                    let context_params = ContextParams::from(&field.attrs).or(&variant.attrs).tokenize();

                    quote! {
                        <#type_name as _opg::OpgModel>::get_schema_with_params(cx, &#context_params)
                    }
                }
                StructStyle::Struct => {
                    object_model(false, &variant.attrs.description, &variant.fields, |field| {
                        variant.attrs.inline || field.attrs.inline
                    })
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
                                data: _opg::ModelData::Single(_opg::ModelType {
                                    nullable: false,
                                    type_description: _opg::ModelTypeDescription::String(_opg::ModelString {
                                        variants: Some(vec![#variant_name.to_owned()]),
                                        data: _opg::ModelSimple {
                                            format: None,
                                            example: Some(#variant_name.to_owned()),
                                        }
                                    })
                                })
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
                        data: _opg::ModelData::Single(_opg::ModelType {
                            nullable: #nullable,
                            type_description: additional_object
                        })
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

fn serialize_struct(container: &Container, fields: &[Field]) -> proc_macro2::TokenStream {
    let description = option_string(container.attrs.description.as_deref());
    let nullable = container.attrs.nullable;

    let object_type_description = object_type_description(fields, |field| field.attrs.inline);

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(_opg::ModelType {
                nullable: #nullable,
                type_description: #object_type_description
            })
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_tuple_struct(container: &Container, fields: &[Field]) -> proc_macro2::TokenStream {
    let description = option_string(container.attrs.description.as_deref());
    let nullable = container.attrs.nullable;

    let tuple_type_description = tuple_type_description(fields, |field| field.attrs.inline);

    let body = quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(_opg::ModelType {
                nullable: #nullable,
                type_description: #tuple_type_description
            })
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn serialize_newtype_struct(container: &Container, field: &Field) -> proc_macro2::TokenStream {
    let type_name = &field.original.ty;

    let context_params = ContextParams::from(&field.attrs).or(&container.attrs);

    let body = match container.attrs.explicit_model_type {
        Some(explicit_model_type) if explicit_model_type != ExplicitModelType::Any => {
            newtype_model(
                container.attrs.nullable,
                context_params,
                explicit_model_type,
            )
        }
        Some(_) => unreachable!(),
        None => {
            let context_params = context_params.tokenize();

            quote! {
                <#type_name as _opg::OpgModel>::get_schema_with_params(cx, &#context_params)
            }
        }
    };

    implement_type(&container.ident, body, container.attrs.inline)
}

fn tuple_model<P>(
    nullable: bool,
    description: &Option<String>,
    fields: &[Field],
    inline_predicate: P,
) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let description = option_string(description.as_deref());
    let tuple_type_description = tuple_type_description(fields, inline_predicate);

    quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(_opg::ModelType {
                nullable: #nullable,
                type_description: #tuple_type_description
            })
        }
    }
}

fn object_model<P>(
    nullable: bool,
    description: &Option<String>,
    fields: &[Field],
    inline_predicate: P,
) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let description = option_string(description.as_deref());
    let object_type_description = object_type_description(fields, inline_predicate);

    quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(_opg::ModelType {
                nullable: #nullable,
                type_description: #object_type_description
            })
        }
    }
}

fn inline_reference(model: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        _opg::ModelReference::Inline(#model)
    }
}

fn tuple_type_description<P>(fields: &[Field], inline_predicate: P) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let data = fields
        .iter()
        .map(|field| {
            field_model_reference(
                ContextParams::from(&field.attrs),
                field,
                inline_predicate(field),
            )
        })
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

fn object_type_description<P>(fields: &[Field], inline_predicate: P) -> proc_macro2::TokenStream
where
    P: Fn(&Field) -> bool,
{
    let data = fields
        .iter()
        .filter(|field| !field.attrs.skip_serializing)
        .map(|field| {
            let field_model_reference = field_model_reference(
                ContextParams::from(&field.attrs),
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

fn field_model_reference<'a>(
    context_params: ContextParams<'a>,
    field: &'a Field,
    inline: bool,
) -> proc_macro2::TokenStream {
    let type_name = &field.original.ty;

    match field.attrs.explicit_model_type {
        Some(explicit_model_type) if explicit_model_type != ExplicitModelType::Any => {
            let model = newtype_model(field.attrs.nullable, context_params, explicit_model_type);

            quote! {
                _opg::ModelReference::Inline(#model)
            }
        }
        Some(_) => {
            quote! {
                _opg::ModelReference::Any
            }
        }
        _ => {
            let context_params = context_params.tokenize();

            quote! {
                cx.mention_schema::<#type_name>(#inline, &#context_params)
            }
        }
    }
}

fn newtype_model(
    nullable: bool,
    context_params: ContextParams,
    explicit_model_type: ExplicitModelType,
) -> proc_macro2::TokenStream {
    let (description, format, example) = context_params.split();

    let data = match explicit_model_type {
        ExplicitModelType::String => quote! {
            _opg::ModelTypeDescription::String(_opg::ModelString {
                variants: None,
                data: _opg::ModelSimple {
                    format: #format,
                    example: #example,
                }
            })
        },
        ExplicitModelType::Integer => quote! {
            _opg::ModelTypeDescription::Integer(_opg::ModelSimple {
                format: #format,
                example: #example,
            })
        },
        ExplicitModelType::Number => quote! {
            _opg::ModelTypeDescription::Number(_opg::ModelSimple {
                format: #format,
                example: #example,
            })
        },
        ExplicitModelType::Boolean => quote! {
            _opg::ModelTypeDescription::Boolean
        },
        ExplicitModelType::Any => unreachable!(),
    };

    quote! {
        _opg::Model {
            description: #description,
            data: _opg::ModelData::Single(_opg::ModelType {
                nullable: #nullable,
                type_description: #data
            }),
        }
    }
}

fn option_to_string_expr(data: Option<&syn::Expr>) -> proc_macro2::TokenStream {
    match data {
        Some(data) => {
            quote! { Some((#data).to_string()) }
        }
        None => quote! { None },
    }
}

fn option_string(data: Option<&str>) -> proc_macro2::TokenStream {
    match data {
        Some(data) => {
            quote! { Some(#data.to_owned()) }
        }
        None => quote! { None },
    }
}

fn option_bool(data: Option<bool>) -> proc_macro2::TokenStream {
    match data {
        Some(data) => {
            quote! { Some(#data) }
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
            #[inline]
            fn type_name() -> Option<std::borrow::Cow<'static, str>> {
                None
            }

            #[inline]
            fn select_reference(cx: &mut _opg::Components, _: bool, params: &_opg::ContextParams) -> _opg::ModelReference {
                _opg::ModelReference::Inline(Self::get_schema(cx).apply_params(params))
            }
        }
    } else {
        quote! {
            #[inline]
            fn type_name() -> Option<std::borrow::Cow<'static, str>> {
                Some(stringify!(#type_name).into())
            }
        }
    };

    quote! {
        fn get_schema(cx: &mut _opg::Components) -> _opg::Model {
            #body
        }

        #inline
    }
}

#[derive(Default, Copy, Clone)]
struct ContextParams<'a> {
    description: Option<&'a str>,
    nullable: Option<bool>,
    format: Option<&'a str>,
    example: Option<&'a syn::Expr>,
}

impl<'a> From<&'a attr::Container> for ContextParams<'a> {
    fn from(attrs: &'a attr::Container) -> Self {
        Self::new()
            .description(attrs.description.as_deref())
            .nullable(if attrs.nullable { Some(true) } else { None })
            .format(attrs.format.as_deref())
            .example(attrs.example.as_ref())
    }
}

impl<'a> From<&'a attr::Variant> for ContextParams<'a> {
    fn from(attrs: &'a attr::Variant) -> Self {
        Self::new()
            .description(attrs.description.as_deref())
            .format(attrs.format.as_deref())
            .example(attrs.example.as_ref())
    }
}

impl<'a> From<&'a attr::Field> for ContextParams<'a> {
    fn from(attrs: &'a attr::Field) -> Self {
        Self::new()
            .description(attrs.description.as_deref())
            .nullable(if attrs.nullable { Some(true) } else { None })
            .format(attrs.format.as_deref())
            .example(attrs.example.as_ref())
    }
}

impl<'a> ContextParams<'a> {
    fn new() -> Self {
        Default::default()
    }

    fn description(mut self, description: Option<&'a str>) -> Self {
        self.description = description;
        self
    }

    fn nullable(mut self, nullable: Option<bool>) -> Self {
        self.nullable = nullable;
        self
    }

    fn format(mut self, format: Option<&'a str>) -> Self {
        self.format = format;
        self
    }

    fn example(mut self, example: Option<&'a syn::Expr>) -> Self {
        self.example = example;
        self
    }

    fn or<T>(mut self, other: T) -> Self
    where
        T: Into<ContextParams<'a>>,
    {
        let other = other.into();
        self.description = self.description.or(other.description);
        self.format = self.format.or(other.format);
        self.example = self.example.or(other.example);
        self
    }

    fn split(
        self,
    ) -> (
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    ) {
        (
            option_string(self.description),
            option_string(self.format),
            option_to_string_expr(self.example),
        )
    }

    fn tokenize(self) -> proc_macro2::TokenStream {
        let description = option_string(self.description);
        let nullable = option_bool(self.nullable);
        let format = option_string(self.format);
        let example = option_to_string_expr(self.example);

        quote! {
            _opg::ContextParams {
                description: #description,
                nullable: #nullable,
                variants: None,
                format: #format,
                example: #example,
            }
        }
    }
}
