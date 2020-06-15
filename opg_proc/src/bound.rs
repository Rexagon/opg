use std::collections::HashSet;

use super::ast::{Container, Data};
use super::attr::{self, Field};

pub fn without_defaults(generics: &syn::Generics) -> syn::Generics {
    syn::Generics {
        params: generics.params.iter().map(|param| match param {
            syn::GenericParam::Type(param) => syn::GenericParam::Type(syn::TypeParam {
                eq_token: None,
                default: None,
                ..param.clone()
            }),
            _ => param.clone(),
        }),
        ..generics.clone()
    }
}

pub fn with_where_predicates(
    generics: &syn::Generics,
    predicates: &[syn::WherePredicate],
) -> syn::Generics {
    let mut generics = generics.clone();
    generics
        .make_where_clause()
        .predicates
        .extend(predicates.iter().cloned());

    generics
}

pub fn with_where_predicate_from_fields(
    container: &Container,
    generics: &syn::Generics,
    from_field: fn(&Field) -> Option<&[syn::WherePredicate]>,
) -> syn::Generics {
    let predicates = container
        .data
        .all_fields()
        .flat_map(|field| from_field(&field.attrs))
        .flat_map(|predicates| predicates.to_vec());

    let mut generics = generics.clone();
    generics.make_where_clause().predicates.extend(predicates);
    generics
}

pub fn with_where_predicates_from_variants(
    cont: &Container,
    generics: &syn::Generics,
    from_variant: fn(&attr::Variant) -> Option<&[syn::WherePredicate]>,
) -> syn::Generics {
    let variants = match &cont.data {
        Data::Enum(variants) => variants,
        Data::Struct(_, _) => {
            return generics.clone();
        }
    };

    let predicates = variants
        .iter()
        .flat_map(|variant| from_variant(&variant.attrs))
        .flat_map(|predicates| predicates.to_vec());

    let mut generics = generics.clone();
    generics.make_where_clause().predicates.extend(predicates);
    generics
}
