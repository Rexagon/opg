use itertools::*;
use syn::punctuated::Punctuated;

use crate::attr;
use crate::parsing_context::*;

pub struct Container<'a> {
    pub ident: syn::Ident,
    pub attrs: attr::Container,
    pub data: Data<'a>,
    pub generics: &'a syn::Generics,
    pub original: &'a syn::DeriveInput,
}

pub enum Data<'a> {
    Enum(Vec<Variant<'a>>),
    Struct(StructStyle, Vec<Field<'a>>),
}

pub struct Variant<'a> {
    pub ident: syn::Ident,
    pub attrs: attr::Variant,
    pub style: StructStyle,
    pub fields: Vec<Field<'a>>,
    pub original: &'a syn::Variant,
}

pub struct Field<'a> {
    pub member: syn::Member,
    pub attrs: attr::Field,
    pub ty: &'a syn::Type,
    pub original: &'a syn::Field,
}

impl<'a> Container<'a> {
    pub fn from_ast(cx: &ParsingContext, input: &'a syn::DeriveInput) -> Option<Container<'a>> {
        let mut attrs = attr::Container::from_ast(cx, input);

        let mut data = match &input.data {
            syn::Data::Enum(data) => Data::Enum(enum_from_ast(cx, &data.variants)),
            syn::Data::Struct(data) => {
                let (style, fields) = struct_from_ast(cx, &data.fields);
                Data::Struct(style, fields)
            }
            syn::Data::Union(_) => {
                cx.error_spanned_by(input, "union types are not supported");
                return None;
            }
        };

        let mut has_flatten = false;
        match &mut data {
            Data::Enum(variants) => {
                for variant in variants {
                    variant.attrs.rename_by_rule(attrs.rename_rule);
                    for field in &mut variant.fields {
                        if field.attrs.flatten {
                            has_flatten = true;
                        }
                        field.attrs.rename_by_rule(attrs.rename_rule);
                    }
                }
            }
            Data::Struct(_, fields) => {
                for field in fields {
                    if field.attrs.flatten {
                        has_flatten = true;
                    }
                    field.attrs.rename_by_rule(attrs.rename_rule);
                }
            }
        }

        if has_flatten {
            attrs.has_flatten = true;
        }

        let item = Self {
            ident: input.ident.clone(),
            attrs,
            data,
            generics: &input.generics,
            original: input,
        };
        // TODO: check item
        Some(item)
    }
}

impl<'a> Data<'a> {
    #[allow(dead_code)]
    pub fn all_fields(&'a self) -> impl Iterator<Item = &'a Field<'a>> {
        match self {
            Data::Enum(variants) => {
                Either::Left(variants.iter().flat_map(|variant| variant.fields.iter()))
            }
            Data::Struct(_, fields) => Either::Right(fields.iter()),
        }
    }
}

fn enum_from_ast<'a>(
    cx: &ParsingContext,
    variants: &'a Punctuated<syn::Variant, syn::Token![,]>,
) -> Vec<Variant<'a>> {
    variants
        .iter()
        .map(|variant| {
            let attrs = attr::Variant::from_ast(cx, variant);
            let (style, fields) = struct_from_ast(cx, &variant.fields);
            Variant {
                ident: variant.ident.clone(),
                attrs,
                style,
                fields,
                original: variant,
            }
        })
        .collect()
}

fn struct_from_ast<'a>(
    cx: &ParsingContext,
    fields: &'a syn::Fields,
) -> (StructStyle, Vec<Field<'a>>) {
    match fields {
        syn::Fields::Named(fields) => (StructStyle::Struct, fields_from_ast(cx, &fields.named)),
        syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            (StructStyle::NewType, fields_from_ast(cx, &fields.unnamed))
        }
        syn::Fields::Unnamed(fields) => (StructStyle::Tuple, fields_from_ast(cx, &fields.unnamed)),
        syn::Fields::Unit => (StructStyle::Unit, Vec::new()),
    }
}

fn fields_from_ast<'a>(
    cx: &ParsingContext,
    fields: &'a Punctuated<syn::Field, syn::Token![,]>,
) -> Vec<Field<'a>> {
    fields
        .iter()
        .enumerate()
        .map(|(i, field)| Field {
            member: match &field.ident {
                Some(ident) => syn::Member::Named(ident.clone()),
                None => syn::Member::Unnamed(i.into()),
            },
            attrs: attr::Field::from_ast(cx, i, field),
            ty: &field.ty,
            original: field,
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub enum StructStyle {
    Struct,
    Tuple,
    NewType,
    Unit,
}
