use std::collections::HashMap;
use std::iter::Peekable;
use std::str::FromStr;
use std::sync::Mutex;

use lazy_static::lazy_static;
use proc_macro2::{Delimiter, TokenStream, TokenTree};

use crate::parsing_context::ParsingContext;

lazy_static! {
    pub static ref PATH_DESCRIPTIONS: Mutex<HashMap<Vec<PathSegment>, HashMap<HttpMethod, PathContent>>> = Mutex::new(Default::default());
}

pub fn impl_path(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> Result<TokenStream, Vec<syn::Error>> {
    let cx = ParsingContext::new();

    let tt = TokenStream::from(attr);
    let parsed = parse_path(&cx, tt.clone()).or_else(report_error(&cx, tt, "failed to generate path description"));

    if let Some((method, path, content)) = parsed {
        let mut path_descriptions = PATH_DESCRIPTIONS.lock().unwrap();
        path_descriptions.entry(path).or_insert_with(HashMap::new).insert(method, content);
    }

    cx.check().map(|_| item.into())
}

fn parse_path(cx: &ParsingContext, attr: TokenStream) -> Option<(HttpMethod, Vec<PathSegment>, PathContent)> {
    let mut iter = attr.into_iter().peekable();

    let http_method = parse_ident(cx, &mut iter).and_then(|ident| {
        HttpMethod::from_str(&ident)
            .ok()
            .or_else(report_error(cx, iter.clone().collect::<TokenStream>(), "invalid http method name"))
    })?;

    let mut content = PathContent::default();

    let path = parse_path_address(cx, &mut iter, &mut content.parameters)?;
    parse_delimiter(cx, &mut iter, ':')?;
    parse_path_content(cx, &mut iter, &mut content)?;

    println!("http method: {:?}", http_method);
    println!("path: {:?}", path);
    println!("content: {:#?}", content);

    Some((http_method, path, content))
}

fn parse_path_content<I>(cx: &ParsingContext, input: &mut I, content: &mut PathContent) -> Option<()>
where
    I: Iterator<Item = TokenTree>,
{
    let mut content_iter = input
        .next()
        .and_then(|tt| match tt {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => Some(group.stream()),
            tt => None.or_else(report_error(cx, tt, "expected `{...}` group")),
        })?
        .into_iter()
        .peekable();

    loop {
        let key = parse_key(cx, &mut content_iter)?;
        parse_delimiter(cx, &mut content_iter, ':')?;

        match key.as_ref() {
            PathContentKeyRef::Ident("tags") => {
                content.tags = parse_group_list(cx, &mut content_iter)?;
            }
            PathContentKeyRef::Ident("summary") => {
                content.summary = Some(parse_string(cx, &mut content_iter)?);
            }
            PathContentKeyRef::Ident("description") => {
                content.description = Some(parse_string(cx, &mut content_iter)?);
            }
            PathContentKeyRef::Ident("security") => {
                content.security = parse_group_list(cx, &mut content_iter)?;
            }
            PathContentKeyRef::Ident("parameters") => {
                content.parameters.extend(parse_path_parameters(cx, &mut content_iter)?);
            }
            PathContentKeyRef::Ident("body") => {
                content.body = Some(parse_type_until(cx, &mut content_iter, ',')?.to_string());
            }
            PathContentKeyRef::Code(code, _description) => {
                let response_model = parse_type_until(cx, &mut content_iter, ',')?;
                content.responses.insert(code, response_model.to_string());
            }
            _ => return None.or_else(report_error(cx, content_iter.collect::<TokenStream>(), "invalid field"))?,
        }

        if parse_trailing_delimiter(cx, &mut content_iter, ',')? {
            break;
        }
    }

    Some(())
}

fn parse_key<I>(cx: &ParsingContext, input: &mut Peekable<I>) -> Option<PathContentKey>
where
    I: Iterator<Item = TokenTree>,
    Peekable<I>: Clone,
{
    let tt = input.clone().collect::<TokenStream>();
    let start_item = input.peek().or_else(report_error(cx, tt, "expected fields"))?;

    match start_item {
        TokenTree::Ident(_) => parse_ident(cx, input).map(PathContentKey::Ident),
        TokenTree::Literal(_) => {
            let code = parse_integer(cx, input)?;
            let mut description = None;
            if let Some(TokenTree::Group(group)) = input.peek() {
                let mut desc_iter = group.stream().into_iter();
                description = Some(parse_string_or_ident(cx, &mut desc_iter)?);
            }
            Some(PathContentKey::Code(code, description))
        }
        tt => None.or_else(report_error(cx, tt, "expected identifier or integer literal")),
    }
}

fn parse_path_address<I>(
    cx: &ParsingContext,
    input: &mut Peekable<I>,
    parameters: &mut HashMap<String, Parameter>,
) -> Option<Vec<PathSegment>>
where
    I: Iterator<Item = TokenTree>,
    Peekable<I>: Clone,
{
    let mut address_iter = input
        .next()
        .and_then(|token| match token {
            TokenTree::Group(group) => Some(group),
            tt => None.or_else(report_error(cx, tt, "expected `(...) group`")),
        })?
        .stream()
        .into_iter()
        .peekable();

    if let Some(TokenTree::Punct(punct)) = address_iter.peek() {
        if punct.as_char() == '/' {
            let _ = address_iter.next();
        }
    }

    let mut result = Vec::new();
    loop {
        match address_iter.peek() {
            Some(tt @ TokenTree::Literal(_)) => {
                let tt = tt.clone();
                let segment = parse_string(cx, &mut address_iter).or_else(report_error(cx, tt, "failed to parse path segment"))?;

                result.push(PathSegment::Path(segment));
            }
            Some(tt @ TokenTree::Ident(_)) => {
                let tt = tt.clone();
                let ty = parse_ident(cx, &mut address_iter).or_else(report_error(cx, tt, "failed to parse parameter"))?;
                let param_name = ty[..1].to_ascii_lowercase() + &ty[1..];

                parameters.insert(param_name.clone(), Parameter::from_path(ty));

                result.push(PathSegment::Parameter(param_name));
            }
            Some(TokenTree::Group(_)) => {
                let (param_name, ty) = parse_path_address_named_segment(cx, &mut address_iter)?;

                parameters.insert(param_name.clone(), Parameter::from_path(ty.to_string()));

                result.push(PathSegment::Parameter(param_name));
            }
            tt => None.or_else(report_error(cx, tt, "expected literal or ident or `{...} group`"))?,
        };

        if parse_trailing_delimiter(cx, &mut address_iter, '/')? {
            break Some(result);
        }
    }
}

fn parse_path_address_named_segment<I>(cx: &ParsingContext, input: &mut Peekable<I>) -> Option<(String, syn::TypePath)>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Group(group) => {
            let mut param_iter = group.stream().into_iter().peekable();

            let param_name = parse_ident(cx, &mut param_iter).or_else(report_error(cx, group, "failed to parse parameter name"))?;

            parse_delimiter(cx, &mut param_iter, ':')?;

            let tt = param_iter.collect::<TokenStream>();
            let param_type = syn::parse2::<syn::TypePath>(tt.clone())
                .ok()
                .or_else(report_error(cx, tt, "invalid type path"))?;

            Some((param_name, param_type))
        }
        _ => None,
    })
}

fn parse_path_parameters<I>(cx: &ParsingContext, input: &mut Peekable<I>) -> Option<HashMap<String, Parameter>>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
            let mut group_iter = group.stream().into_iter().peekable();

            let mut result = HashMap::new();

            loop {
                let (name, parameter) = parse_path_parameter_item(cx, &mut group_iter)?;
                result.insert(name, parameter);

                if parse_trailing_delimiter(cx, &mut group_iter, ',')? {
                    break Some(result);
                }
            }
        }
        tt => None.or_else(report_error(cx, tt, "expected `{...}` group")),
    })
}

fn parse_path_parameter_item<I>(cx: &ParsingContext, input: &mut I) -> Option<(String, Parameter)>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
            let mut group_iter = group.stream().into_iter().peekable();

            let parameter_in = parse_ident(cx, &mut group_iter)
                .and_then(|ident| ParameterIn::from_str(&ident).ok())
                .or_else(report_error(cx, group.clone(), "failed to parse `ParameterIn`"))?;

            let name = parse_ident(cx, &mut group_iter).or_else(report_error(cx, group, "failed to parse parameter name"))?;

            parse_delimiter(cx, &mut group_iter, ':')?;

            let tt = group_iter.collect::<TokenStream>();
            let ty = syn::parse2::<syn::TypePath>(tt.clone())
                .ok()
                .or_else(report_error(cx, tt, "invalid type path"))?;

            Some((
                name,
                Parameter {
                    description: None,
                    parameter_in,
                    required: parameter_in.required_by_default(),
                    ty: ty.to_string(),
                },
            ))
        }
        tt => None.or_else(report_error(cx, tt, "expected `(...)` group")),
    })
}

fn parse_type_until<I>(cx: &ParsingContext, input: &mut Peekable<I>, delimiter: char) -> Option<syn::TypePath>
where
    I: Iterator<Item = TokenTree>,
    Peekable<I>: Clone,
{
    let tt = input
        .clone()
        .take_while(|_| match input.peek() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == delimiter => false,
            _ => input.next().is_some(),
        })
        .collect::<TokenStream>();

    syn::parse2::<syn::TypePath>(tt.clone())
        .ok()
        .or_else(report_error(cx, tt, "invalid type path"))
}

fn parse_group_list<I>(cx: &ParsingContext, input: &mut I) -> Option<Vec<String>>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Group(group) if matches!(group.delimiter(), Delimiter::Brace | Delimiter::Bracket) => {
            let mut group_iter = group.stream().into_iter().peekable();
            let mut result = Vec::new();
            loop {
                result.push(parse_string_or_ident(cx, &mut group_iter)?);
                if parse_trailing_delimiter(cx, &mut group_iter, ',')? {
                    break Some(result);
                }
            }
        }
        tt @ TokenTree::Ident(_) => syn::parse2::<syn::Ident>(tt.clone().into())
            .ok()
            .or_else(report_error(cx, tt, "invalid identifier"))
            .map(|ident| vec![ident.to_string()]),
        tt @ TokenTree::Literal(_) => syn::parse2::<syn::LitStr>(tt.clone().into())
            .ok()
            .or_else(report_error(cx, tt, "invalid string literal"))
            .map(|literal| vec![literal.value()]),
        tt => None.or_else(report_error(cx, tt, "expected group or identifier or string literal")),
    })
}

fn parse_string_or_ident<I>(cx: &ParsingContext, input: &mut I) -> Option<String>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        tt @ TokenTree::Ident(_) => syn::parse2::<syn::Ident>(tt.clone().into())
            .ok()
            .or_else(report_error(cx, tt, "invalid identifier"))
            .map(|ident| ident.to_string()),
        tt @ TokenTree::Literal(_) => syn::parse2::<syn::LitStr>(tt.clone().into())
            .ok()
            .or_else(report_error(cx, tt, "invalid string literal"))
            .map(|literal| literal.value()),
        tt => None.or_else(report_error(cx, tt, "expected identifier or string literal")),
    })
}

fn parse_string<I>(cx: &ParsingContext, input: &mut I) -> Option<String>
where
    I: Iterator<Item = TokenTree>,
{
    input
        .next()
        .and_then(|tt| {
            syn::parse2::<syn::LitStr>(tt.clone().into())
                .ok()
                .or_else(report_error(cx, tt, "invalid string literal"))
        })
        .map(|literal: syn::LitStr| literal.value())
}

fn parse_ident<I>(cx: &ParsingContext, input: &mut I) -> Option<String>
where
    I: Iterator<Item = TokenTree>,
{
    input
        .next()
        .and_then(|tt| {
            syn::parse2::<syn::Ident>(tt.clone().into())
                .ok()
                .or_else(report_error(cx, tt, "invalid identifier"))
        })
        .map(|ident| ident.to_string())
}

fn parse_integer<I, T>(cx: &ParsingContext, input: &mut I) -> Option<T>
where
    I: Iterator<Item = TokenTree>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    input
        .next()
        .and_then(|tt| {
            syn::parse2::<syn::LitInt>(tt.clone().into())
                .ok()
                .or_else(report_error(cx, tt, "invalid integer literal"))
        })
        .and_then(|literal| {
            literal.base10_parse().ok().or_else(report_error(
                cx,
                literal,
                format!("invalid integer literal value (`{}` expected)", std::any::type_name::<T>()),
            ))
        })
}

fn parse_trailing_delimiter<I>(cx: &ParsingContext, input: &mut Peekable<I>, delimiter: char) -> Option<bool>
where
    I: Iterator<Item = TokenTree>,
{
    if input.peek().is_some() {
        parse_delimiter(cx, input, delimiter)?;
    }
    Some(input.peek().is_none())
}

fn parse_delimiter<I>(cx: &ParsingContext, input: &mut I, delimiter: char) -> Option<()>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Punct(punct) if punct.as_char() == delimiter => Some(()),
        TokenTree::Punct(punct) => None.or_else(report_error(cx, punct, format!("invalid punctuation (`{}` expected)", delimiter))),
        tt => None.or_else(report_error(cx, tt, format!("`{}` expected", delimiter))),
    })
}

#[derive(Debug)]
pub enum PathContentKey {
    Ident(String),
    Code(u16, Option<String>),
}

impl PathContentKey {
    fn as_ref(&self) -> PathContentKeyRef {
        match self {
            Self::Ident(ident) => PathContentKeyRef::Ident(ident),
            Self::Code(code, description) => PathContentKeyRef::Code(*code, description.as_ref().map(String::as_str)),
        }
    }
}

enum PathContentKeyRef<'a> {
    Ident(&'a str),
    Code(u16, Option<&'a str>),
}

#[derive(Debug, Default)]
pub struct PathContent {
    pub tags: Vec<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub security: Vec<String>,
    pub parameters: HashMap<String, Parameter>,
    pub body: Option<String>,
    pub responses: HashMap<u16, String>,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum PathSegment {
    Path(String),
    Parameter(String),
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub description: Option<String>,
    pub parameter_in: ParameterIn,
    pub required: bool,
    pub ty: String,
}

impl Parameter {
    fn from_path(ty: String) -> Self {
        Self {
            description: None,
            parameter_in: ParameterIn::Path,
            required: true,
            ty,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}

impl ParameterIn {
    fn required_by_default(&self) -> bool {
        match self {
            Self::Query | Self::Header | Self::Cookie => false,
            Self::Path => true,
        }
    }
}

impl std::fmt::Display for ParameterIn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Query => "query",
            Self::Header => "header",
            Self::Path => "path",
            Self::Cookie => "cookie",
        })
    }
}

impl FromStr for ParameterIn {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "query" => Self::Query,
            "header" => Self::Header,
            "path" => Self::Path,
            "cookie" => Self::Cookie,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum HttpMethod {
    GET,
    PUT,
    POST,
    DELETE,
    OPTIONS,
    HEAD,
    PATCH,
    TRACE,
}

impl FromStr for HttpMethod {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "GET" => HttpMethod::GET,
            "PUT" => HttpMethod::PUT,
            "POST" => HttpMethod::POST,
            "DELETE" => HttpMethod::DELETE,
            "OPTIONS" => HttpMethod::OPTIONS,
            "HEAD" => HttpMethod::HEAD,
            "PATCH" => HttpMethod::PATCH,
            "TRACE" => HttpMethod::TRACE,
            _ => return Err(()),
        })
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            HttpMethod::GET => "GET",
            HttpMethod::PUT => "PUT",
            HttpMethod::POST => "POST",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::TRACE => "TRACE",
        })
    }
}

trait TypePathExt {
    fn to_string(&self) -> String;
}

impl TypePathExt for syn::TypePath {
    fn to_string(&self) -> String {
        stream_to_string(&self)
    }
}

fn report_error<'c, O, T, R>(cx: &'c ParsingContext, tt: O, message: T) -> impl FnOnce() -> Option<R> + 'c
where
    O: quote::ToTokens + 'c,
    T: std::fmt::Display + 'c,
{
    move || {
        cx.error_spanned_by(tt, message);
        None::<R>
    }
}

fn stream_to_string<T>(ty: &T) -> String
where
    T: quote::ToTokens,
{
    ty.to_token_stream().to_string().replace(' ', "")
}
