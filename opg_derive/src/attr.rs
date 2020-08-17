use std::str::FromStr;

use proc_macro2::{Group, Span, TokenStream, TokenTree};
use syn::export::{Formatter, ToTokens};
use syn::punctuated::Punctuated;
use syn::Meta::*;
use syn::NestedMeta::*;

use crate::case::*;
use crate::dummy::*;
use crate::parsing_context::*;
use crate::symbol::*;

pub struct Container {
    pub name: Name,
    pub rename_rule: RenameRule,
    pub transparent: bool,
    pub tag_type: TagType,
    pub has_flatten: bool,
    pub has_repr: bool,

    pub description: Option<String>,
    pub format: Option<String>,
    pub example: Option<String>,
    pub inline: bool,
    pub nullable: bool,
    pub explicit_model_type: Option<ExplicitModelType>,
    pub model_type: ModelType,
}

impl Container {
    #[allow(clippy::cognitive_complexity)]
    pub fn from_ast(cx: &ParsingContext, input: &syn::DeriveInput) -> Self {
        let mut ser_name = Attr::none(cx, RENAME);
        let mut rename_rule = Attr::none(cx, RENAME_ALL);
        let mut transparent = BoolAttr::none(cx, TRANSPARENT);
        let mut untagged = BoolAttr::none(cx, UNTAGGED);
        let mut internal_tag = Attr::none(cx, TAG);
        let mut content = Attr::none(cx, CONTENT);
        let mut has_repr = BoolAttr::none(cx, REPR);

        let mut description = Attr::none(cx, DESCRIPTION);
        let mut format = Attr::none(cx, FORMAT);
        let mut example = Attr::none(cx, EXAMPLE);
        let mut inline = BoolAttr::none(cx, INLINE);
        let mut nullable = BoolAttr::none(cx, NULLABLE);
        let mut model_type = OneOfFlagsAttr::none(cx);

        for (from, meta_item) in input
            .attrs
            .iter()
            .flat_map(|attr| get_meta_items(cx, attr))
            .flat_map(|item| item.into_iter())
        {
            match (from, &meta_item) {
                (AttrFrom::Serde, Meta(NameValue(m))) if m.path == RENAME => {
                    if let Ok(s) = get_lit_str(cx, RENAME, &m.lit) {
                        ser_name.set(&m.path, s.value());
                    }
                }
                (AttrFrom::Serde, Meta(List(m))) if m.path == RENAME => {
                    if let Ok(ser) = get_renames(cx, &m.nested) {
                        ser_name.set_opt(&m.path, ser.map(syn::LitStr::value));
                    }
                }
                (AttrFrom::Serde, Meta(NameValue(m))) if m.path == RENAME_ALL => {
                    if let Ok(rule) = get_lit_str(cx, RENAME_ALL, &m.lit)
                        .and_then(|s| RenameRule::from_str(&s.value()))
                    {
                        rename_rule.set(&m.path, rule)
                    }
                }
                (AttrFrom::Serde, Meta(List(m))) if m.path == RENAME_ALL => {
                    if let Ok(Some(rule)) = get_renames(cx, &m.nested) {
                        if let Ok(rule) = RenameRule::from_str(&rule.value()) {
                            rename_rule.set(&m.path, rule)
                        }
                    }
                }
                (AttrFrom::Serde, Meta(Path(word))) if word == TRANSPARENT => {
                    transparent.set_true(word);
                }
                (AttrFrom::Serde, Meta(Path(word))) if word == UNTAGGED => {
                    if let syn::Data::Enum(_) = input.data {
                        untagged.set_true(word);
                    }
                }
                (AttrFrom::Serde, Meta(NameValue(m))) if m.path == TAG => {
                    if let Ok(s) = get_lit_str_simple(&m.lit) {
                        match &input.data {
                            syn::Data::Enum(_)
                            | syn::Data::Struct(syn::DataStruct {
                                fields: syn::Fields::Named(_),
                                ..
                            }) => {
                                internal_tag.set(&m.path, s.value());
                            }
                            _ => {}
                        }
                    }
                }
                (AttrFrom::Serde, Meta(NameValue(m))) if m.path == CONTENT => {
                    if let Ok(s) = get_lit_str_simple(&m.lit) {
                        if let syn::Data::Enum(_) = &input.data {
                            content.set(&m.path, s.value());
                        }
                    }
                }
                (AttrFrom::Serde, _) => {}
                (AttrFrom::Opg, Lit(lit)) => {
                    if let Ok(s) = get_lit_str_simple(lit) {
                        description.set(lit, s.value().clone());
                    }
                }
                (AttrFrom::Opg, Meta(NameValue(m))) if m.path == FORMAT => {
                    if let Ok(s) = get_lit_str(cx, FORMAT, &m.lit) {
                        format.set(&m.path, s.value().clone())
                    }
                }
                (AttrFrom::Opg, Meta(NameValue(m))) if m.path == EXAMPLE => {
                    if let Ok(s) = get_lit_str(cx, EXAMPLE, &m.lit) {
                        example.set(&m.path, s.value().clone())
                    }
                }
                (AttrFrom::Opg, Meta(Path(word))) if word == INLINE => inline.set_true(word),
                (AttrFrom::Opg, Meta(Path(word))) if word == NULLABLE => nullable.set_true(word),
                (AttrFrom::Opg, Meta(Path(word))) => {
                    if let Ok(t) = ExplicitModelType::from_path(word) {
                        model_type.set(word, t);
                    } else {
                        cx.error_spanned_by(word, "unknown attribute")
                    }
                }
                (AttrFrom::Opg, Meta(meta_item)) => {
                    let path = meta_item
                        .path()
                        .into_token_stream()
                        .to_string()
                        .replace(' ', "");
                    cx.error_spanned_by(
                        meta_item.path(),
                        format!("unknown opg variant attribute `{}`", path),
                    );
                }
                (AttrFrom::Repr, item) => {
                    has_repr.set_true(item);
                }
            }
        }

        let tag_type = decide_tag(untagged, internal_tag, content);
        let explicit_model_type = model_type.at_most_one();
        let model_type = decide_model_type(cx, input, &tag_type).unwrap_or(ModelType::Object);

        Self {
            name: Name::from_attrs(unraw(&input.ident), ser_name),
            rename_rule: rename_rule.get().unwrap_or(RenameRule::None),
            transparent: transparent.get(),
            tag_type,
            has_flatten: false,
            has_repr: has_repr.get(),
            description: description.get(),
            format: format.get(),
            example: example.get(),
            inline: inline.get(),
            nullable: nullable.get(),
            explicit_model_type,
            model_type,
        }
    }
}

pub struct Variant {
    pub name: Name,
    pub rename_rule: RenameRule,
    pub skip_serializing: bool,

    pub description: Option<String>,
    pub format: Option<String>,
    pub example: Option<String>,
    pub inline: bool,
    pub explicit_model_type: Option<ExplicitModelType>,
}

impl Variant {
    pub fn from_ast(cx: &ParsingContext, input: &syn::Variant) -> Self {
        let mut ser_name = Attr::none(cx, RENAME);
        let mut rename_rule = Attr::none(cx, RENAME_ALL);
        let mut skip_serializing = BoolAttr::none(cx, SKIP_SERIALIZING);

        let mut description = Attr::none(cx, DESCRIPTION);
        let mut format = Attr::none(cx, FORMAT);
        let mut example = Attr::none(cx, EXAMPLE);
        let mut inline = BoolAttr::none(cx, INLINE);
        let mut model_type = OneOfFlagsAttr::none(cx);

        for (from, meta_item) in input
            .attrs
            .iter()
            .flat_map(|attr| get_meta_items(cx, attr))
            .flat_map(|item| item.into_iter())
        {
            match (from, &meta_item) {
                (AttrFrom::Serde, Meta(NameValue(m))) if m.path == RENAME => {
                    if let Ok(s) = get_lit_str(cx, RENAME, &m.lit) {
                        ser_name.set(&m.path, s.value());
                    }
                }
                (AttrFrom::Serde, Meta(List(m))) if m.path == RENAME => {
                    if let Ok(ser) = get_renames(cx, &m.nested) {
                        ser_name.set_opt(&m.path, ser.map(syn::LitStr::value));
                    }
                }
                (AttrFrom::Serde, Meta(NameValue(m))) if m.path == RENAME_ALL => {
                    if let Ok(rule) = get_lit_str(cx, RENAME_ALL, &m.lit)
                        .and_then(|s| RenameRule::from_str(&s.value()))
                    {
                        rename_rule.set(&m.path, rule)
                    }
                }
                (AttrFrom::Serde, Meta(List(m))) if m.path == RENAME_ALL => {
                    if let Ok(Some(rule)) = get_renames(cx, &m.nested) {
                        if let Ok(rule) = RenameRule::from_str(&rule.value()) {
                            rename_rule.set(&m.path, rule)
                        }
                    }
                }
                (AttrFrom::Serde, Meta(Path(word))) if word == SKIP || word == SKIP_SERIALIZING => {
                    skip_serializing.set_true(word);
                }
                (AttrFrom::Serde, _) => {}
                (AttrFrom::Opg, Lit(lit)) => {
                    if let Ok(s) = get_lit_str_simple(lit) {
                        description.set(lit, s.value().clone());
                    }
                }
                (AttrFrom::Opg, Meta(NameValue(m))) if m.path == FORMAT => {
                    if let Ok(s) = get_lit_str(cx, FORMAT, &m.lit) {
                        format.set(&m.path, s.value().clone())
                    }
                }
                (AttrFrom::Opg, Meta(NameValue(m))) if m.path == EXAMPLE => {
                    if let Ok(s) = get_lit_str(cx, EXAMPLE, &m.lit) {
                        example.set(&m.path, s.value().clone())
                    }
                }
                (AttrFrom::Opg, Meta(Path(word))) if word == INLINE => inline.set_true(word),
                (AttrFrom::Opg, Meta(Path(word))) => {
                    if let Ok(t) = ExplicitModelType::from_path(word) {
                        model_type.set(word, t);
                    } else {
                        cx.error_spanned_by(word, "unknown attribute")
                    }
                }
                (AttrFrom::Opg, Meta(meta_item)) => {
                    let path = meta_item
                        .path()
                        .into_token_stream()
                        .to_string()
                        .replace(' ', "");
                    cx.error_spanned_by(
                        meta_item.path(),
                        format!("unknown opg variant attribute `{}`", path),
                    );
                }
                (AttrFrom::Repr, _) => {}
            }
        }

        Variant {
            name: Name::from_attrs(unraw(&input.ident), ser_name),
            rename_rule: rename_rule.get().unwrap_or(RenameRule::None),
            skip_serializing: skip_serializing.get(),
            description: description.get(),
            format: format.get(),
            example: example.get(),
            inline: inline.get(),
            explicit_model_type: model_type.at_most_one(),
        }
    }

    pub fn rename_by_rule(&mut self, rule: RenameRule) {
        self.name.rename_as_variant(rule);
    }
}

pub struct Field {
    pub name: Name,
    pub skip_serializing: bool,
    pub flatten: bool,
    pub transparent: bool,

    pub optional: bool,
    pub description: Option<String>,
    pub format: Option<String>,
    pub example: Option<String>,
    pub inline: bool,
    pub nullable: bool,
    pub explicit_model_type: Option<ExplicitModelType>,
}

impl Field {
    pub fn from_ast(cx: &ParsingContext, index: usize, input: &syn::Field) -> Self {
        let mut ser_name = Attr::none(cx, RENAME);
        let mut skip_serializing = BoolAttr::none(cx, SKIP_SERIALIZING);
        let mut skip_serializing_if = Attr::none(cx, SKIP_SERIALIZING_IF);
        let mut flatten = BoolAttr::none(cx, FLATTEN);

        let mut optional = BoolAttr::none(cx, OPTIONAL);
        let mut description = Attr::none(cx, DESCRIPTION);
        let mut format = Attr::none(cx, FORMAT);
        let mut example = Attr::none(cx, EXAMPLE);
        let mut inline = BoolAttr::none(cx, INLINE);
        let mut nullable = BoolAttr::none(cx, NULLABLE);
        let mut model_type = OneOfFlagsAttr::none(cx);

        let ident = match &input.ident {
            Some(ident) => unraw(ident),
            None => index.to_string(),
        };

        for (from, meta_item) in input
            .attrs
            .iter()
            .flat_map(|attr| get_meta_items(cx, attr))
            .flat_map(|item| item.into_iter())
        {
            match (from, &meta_item) {
                (AttrFrom::Serde, Meta(NameValue(m))) if m.path == RENAME => {
                    if let Ok(s) = get_lit_str(cx, RENAME, &m.lit) {
                        ser_name.set(&m.path, s.value());
                    }
                }
                (AttrFrom::Serde, Meta(List(m))) if m.path == RENAME => {
                    if let Ok(ser) = get_renames(cx, &m.nested) {
                        ser_name.set_opt(&m.path, ser.map(syn::LitStr::value));
                    }
                }
                (AttrFrom::Serde, Meta(Path(word))) if word == SKIP || word == SKIP_SERIALIZING => {
                    skip_serializing.set_true(word);
                }
                (AttrFrom::Serde, Meta(NameValue(m))) if m.path == SKIP_SERIALIZING_IF => {
                    if let Ok(path) = parse_lit_into_expr_path(cx, SKIP_SERIALIZING_IF, &m.lit) {
                        skip_serializing_if.set(&m.path, path);
                    }
                }
                (AttrFrom::Serde, Meta(Path(word))) if word == FLATTEN => {
                    flatten.set_true(word);
                }
                (AttrFrom::Serde, _) => {}
                (AttrFrom::Opg, Lit(lit)) => {
                    if let Ok(s) = get_lit_str_simple(lit) {
                        description.set(lit, s.value().clone());
                    }
                }
                (AttrFrom::Opg, Meta(NameValue(m))) if m.path == FORMAT => {
                    if let Ok(s) = get_lit_str(cx, FORMAT, &m.lit) {
                        format.set(&m.path, s.value().clone())
                    }
                }
                (AttrFrom::Opg, Meta(NameValue(m))) if m.path == EXAMPLE => {
                    if let Ok(s) = get_lit_str(cx, EXAMPLE, &m.lit) {
                        example.set(&m.path, s.value().clone())
                    }
                }
                (AttrFrom::Opg, Meta(Path(word))) if word == OPTIONAL => optional.set_true(word),
                (AttrFrom::Opg, Meta(Path(word))) if word == INLINE => inline.set_true(word),
                (AttrFrom::Opg, Meta(Path(word))) if word == NULLABLE => nullable.set_true(word),
                (AttrFrom::Opg, Meta(Path(word))) => {
                    if let Ok(t) = ExplicitModelType::from_path(word) {
                        model_type.set(word, t);
                    } else {
                        cx.error_spanned_by(word, "unknown attribute")
                    }
                }
                (AttrFrom::Opg, Meta(meta_item)) => {
                    let path = meta_item
                        .path()
                        .into_token_stream()
                        .to_string()
                        .replace(' ', "");
                    cx.error_spanned_by(
                        meta_item.path(),
                        format!("unknown opg variant attribute `{}`", path),
                    );
                }
                (AttrFrom::Repr, _) => {}
            }
        }

        Self {
            name: Name::from_attrs(ident, ser_name),
            skip_serializing: skip_serializing.get(),
            flatten: flatten.get(),
            transparent: false,
            optional: skip_serializing_if.get().is_some() || optional.get(),
            description: description.get(),
            format: format.get(),
            example: example.get(),
            inline: inline.get(),
            nullable: nullable.get(),
            explicit_model_type: model_type.at_most_one(),
        }
    }

    pub fn rename_by_rule(&mut self, rule: RenameRule) {
        self.name.rename_as_field(rule);
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ExplicitModelType {
    String,
    Integer,
    Number,
    Boolean,
    Any,
}

impl ExplicitModelType {
    fn from_path(p: &syn::Path) -> Result<Self, ()> {
        // can't use match here ;(
        if p == STRING {
            Ok(ExplicitModelType::String)
        } else if p == NUMBER {
            Ok(ExplicitModelType::Number)
        } else if p == INTEGER {
            Ok(ExplicitModelType::Integer)
        } else if p == BOOLEAN {
            Ok(ExplicitModelType::Boolean)
        } else if p == ANY {
            Ok(ExplicitModelType::Any)
        } else {
            Err(())
        }
    }
}

#[derive(Copy, Clone)]
pub enum ModelType {
    NewType,
    Object,
    OneOf,
    Dictionary,
}

pub enum TagType {
    External,
    Internal { tag: String },
    Adjacent { tag: String, content: String },
    None,
}

fn decide_tag(untagged: BoolAttr, internal_tag: Attr<String>, content: Attr<String>) -> TagType {
    match (
        untagged.0.get_with_tokens(),
        internal_tag.get_with_tokens(),
        content.get_with_tokens(),
    ) {
        (None, None, None) => TagType::External,
        (Some(_), None, None) => TagType::None,
        (None, Some((_, tag)), None) => TagType::Internal { tag },
        (None, Some((_, tag)), Some((_, content))) => TagType::Adjacent { tag, content },
        _ => TagType::External, // should be an error, but serde will handle it
    }
}

fn decide_model_type(
    cx: &ParsingContext,
    input: &syn::DeriveInput,
    tag_type: &TagType,
) -> Result<ModelType, ()> {
    Ok(match &input.data {
        syn::Data::Enum(variants) => {
            if variants
                .variants
                .iter()
                .all(|field| matches!(field.fields, syn::Fields::Unit))
            {
                return match tag_type {
                    TagType::None => {
                        cx.error_spanned_by(
                            &input.ident,
                            "unit enums are not supported for untagged",
                        );
                        Err(())
                    }
                    _ => Ok(ModelType::NewType),
                };
            }

            match tag_type {
                TagType::None | TagType::Internal { .. } => ModelType::OneOf,
                TagType::External => ModelType::Dictionary,
                TagType::Adjacent { .. } => ModelType::Object,
            }
        }
        syn::Data::Struct(syn::DataStruct { fields, .. }) => match fields {
            syn::Fields::Named(_) => ModelType::Object,
            syn::Fields::Unnamed(_) => ModelType::NewType,
            syn::Fields::Unit => {
                cx.error_spanned_by(&input.ident, "unit structs are not supported");
                return Err(());
            }
        },
        _ => {
            cx.error_spanned_by(&input.ident, "unions are not supported");
            return Err(());
        }
    })
}

fn get_renames<'a>(
    cx: &ParsingContext,
    items: &'a Punctuated<syn::NestedMeta, syn::Token![,]>,
) -> Result<Option<&'a syn::LitStr>, ()> {
    let ser = get_ser(cx, RENAME, items)?;
    Ok(ser.at_most_one()?)
}

fn get_ser<'c, 'm>(
    cx: &'c ParsingContext,
    attr_name: Symbol,
    metas: &'m Punctuated<syn::NestedMeta, syn::Token![,]>,
) -> Result<VecAttr<'c, &'m syn::LitStr>, ()> {
    let mut ser_meta = VecAttr::none(cx, attr_name);

    for meta in metas {
        match meta {
            Meta(NameValue(m)) if m.path == SERIALIZE => {
                if let Ok(value) = get_lit_str_simple(&m.lit) {
                    ser_meta.insert(&m.path, value);
                }
            }
            Meta(NameValue(m)) if m.path == DESERIALIZE => {}
            _ => return Err(()),
        }
    }

    Ok(ser_meta)
}

fn parse_lit_into_expr_path(
    cx: &ParsingContext,
    attr_name: Symbol,
    lit: &syn::Lit,
) -> Result<syn::ExprPath, ()> {
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

fn get_lit_str_simple(lit: &syn::Lit) -> Result<&syn::LitStr, ()> {
    if let syn::Lit::Str(lit) = lit {
        Ok(lit)
    } else {
        Err(())
    }
}

fn get_lit_str<'a>(
    cx: &ParsingContext,
    attr_name: Symbol,
    lit: &'a syn::Lit,
) -> Result<&'a syn::LitStr, ()> {
    get_lit_str_special(cx, attr_name, attr_name, lit)
}

fn get_lit_str_special<'a>(
    cx: &ParsingContext,
    attr_name: Symbol,
    path_name: Symbol,
    lit: &'a syn::Lit,
) -> Result<&'a syn::LitStr, ()> {
    if let syn::Lit::Str(lit) = lit {
        Ok(lit)
    } else {
        cx.error_spanned_by(
            lit,
            format!(
                "expected {} attribute to be a string: `{} = \"...\"`",
                attr_name, path_name
            ),
        );
        Err(())
    }
}

fn get_meta_items(
    cx: &ParsingContext,
    attr: &syn::Attribute,
) -> Result<Vec<(AttrFrom, syn::NestedMeta)>, ()> {
    let attr_from = if attr.path == OPG {
        AttrFrom::Opg
    } else if attr.path == SERDE {
        AttrFrom::Serde
    } else if attr.path == REPR {
        AttrFrom::Repr
    } else {
        return Ok(Vec::new());
    };

    match attr.parse_meta() {
        Ok(List(meta)) => Ok(meta
            .nested
            .into_iter()
            .map(|meta| (attr_from, meta))
            .collect()),
        Ok(other) => {
            cx.error_spanned_by(other, format!("expected #[{}(...)]", attr_from));
            Err(())
        }
        Err(err) => {
            cx.syn_error(err);
            Err(())
        }
    }
}

#[derive(Copy, Clone)]
enum AttrFrom {
    Serde,
    Opg,
    Repr,
}

impl std::fmt::Display for AttrFrom {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AttrFrom::Serde => f.write_str(SERDE.inner()),
            AttrFrom::Opg => f.write_str(OPG.inner()),
            AttrFrom::Repr => f.write_str(REPR.inner()),
        }
    }
}

struct Attr<'c, T> {
    cx: &'c ParsingContext,
    name: Symbol,
    tokens: TokenStream,
    value: Option<T>,
}

impl<'c, T> Attr<'c, T> {
    fn none(cx: &'c ParsingContext, name: Symbol) -> Self {
        Attr {
            cx,
            name,
            tokens: TokenStream::new(),
            value: None,
        }
    }

    fn set<A: ToTokens>(&mut self, object: A, value: T) {
        let tokens = object.into_token_stream();

        if self.value.is_some() {
            self.cx
                .error_spanned_by(tokens, format!("duplicate opg attribute `{}`", self.name));
        } else {
            self.tokens = tokens;
            self.value = Some(value);
        }
    }

    #[allow(dead_code)]
    fn set_opt<A: ToTokens>(&mut self, object: A, value: Option<T>) {
        if let Some(value) = value {
            self.set(object, value);
        }
    }

    #[allow(dead_code)]
    fn set_if_none(&mut self, value: T) {
        if self.value.is_none() {
            self.value = Some(value);
        }
    }

    fn get(self) -> Option<T> {
        self.value
    }

    fn get_with_tokens(self) -> Option<(TokenStream, T)> {
        match self.value {
            Some(value) => Some((self.tokens, value)),
            None => None,
        }
    }
}

struct BoolAttr<'c>(Attr<'c, ()>);

impl<'c> BoolAttr<'c> {
    fn none(cx: &'c ParsingContext, name: Symbol) -> Self {
        BoolAttr(Attr::none(cx, name))
    }

    fn set_true<A: ToTokens>(&mut self, object: A) {
        self.0.set(object, ());
    }

    fn get(&self) -> bool {
        self.0.value.is_some()
    }
}

struct OneOfFlagsAttr<'c, T> {
    cx: &'c ParsingContext,
    first_dup_tokens: TokenStream,
    values: Vec<T>,
}

#[allow(dead_code)]
impl<'c, T> OneOfFlagsAttr<'c, T> {
    fn none(cx: &'c ParsingContext) -> Self {
        OneOfFlagsAttr {
            cx,
            first_dup_tokens: TokenStream::new(),
            values: Vec::new(),
        }
    }

    fn set<A: ToTokens>(&mut self, object: A, value: T) {
        if self.values.len() == 1 {
            self.first_dup_tokens = object.into_token_stream();
        }
        self.values.push(value)
    }

    fn at_most_one(mut self) -> Option<T> {
        if self.values.len() > 1 {
            let dup_token = self.first_dup_tokens;
            self.cx
                .error_spanned_by(dup_token, "duplicate opg attribute");
        }

        self.values.pop()
    }

    fn get(self) -> Vec<T> {
        self.values
    }
}

struct VecAttr<'c, T> {
    cx: &'c ParsingContext,
    name: Symbol,
    first_dup_tokens: TokenStream,
    values: Vec<T>,
}

#[allow(dead_code)]
impl<'c, T> VecAttr<'c, T> {
    fn none(cx: &'c ParsingContext, name: Symbol) -> Self {
        VecAttr {
            cx,
            name,
            first_dup_tokens: TokenStream::new(),
            values: Vec::new(),
        }
    }

    fn insert<A: ToTokens>(&mut self, object: A, value: T) {
        if self.values.len() == 1 {
            self.first_dup_tokens = object.into_token_stream();
        }
        self.values.push(value)
    }

    fn at_most_one(mut self) -> Result<Option<T>, ()> {
        if self.values.len() > 1 {
            let dup_token = self.first_dup_tokens;
            self.cx.error_spanned_by(
                dup_token,
                format!("duplicate opg attribute `{}`", self.name),
            );
            Err(())
        } else {
            Ok(self.values.pop())
        }
    }

    fn get(self) -> Vec<T> {
        self.values
    }
}

pub struct Name {
    source_name: String,
    serialized_name: String,
    renamed: bool,
}

#[allow(dead_code)]
impl Name {
    fn from_attrs(source_name: String, serialized_name: Attr<String>) -> Self {
        let serialized_name = serialized_name.get();
        let renamed = serialized_name.is_some();

        Self {
            source_name: source_name.clone(),
            serialized_name: serialized_name.unwrap_or_else(|| source_name.clone()),
            renamed,
        }
    }

    pub fn rename_as_variant(&mut self, rename_rule: RenameRule) {
        if !self.renamed {
            self.serialized_name = rename_rule.apply_to_variant(&self.source_name);
        }
    }

    pub fn rename_as_field(&mut self, rename_rule: RenameRule) {
        if !self.renamed {
            self.serialized_name = rename_rule.apply_to_field(&self.source_name);
        }
    }

    pub fn raw(&self) -> String {
        self.source_name.clone()
    }

    pub fn serialized(&self) -> String {
        self.serialized_name.clone()
    }
}
