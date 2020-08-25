use std::collections::HashMap;
use std::iter::Peekable;
use std::str::FromStr;

use proc_macro::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;

pub fn impl_path(
    attr: TokenStream,
    item: TokenStream,
) -> Result<proc_macro2::TokenStream, Vec<syn::Error>> {
    let _parsed = parse_path(attr);

    Ok(item.into())
}

fn parse_path(attr: TokenStream) -> Option<(HttpMethod, Vec<PathSegment>, PathContent)> {
    let mut iter = attr.into_iter().peekable();

    let http_method = parse_ident(&mut iter).and_then(|ident| HttpMethod::from_str(&ident).ok())?;

    let mut content = PathContent::default();

    let path = parse_path_address(&mut iter, &mut content.parameters)?;
    parse_delimiter(&mut iter, ':')?;
    parse_path_content(&mut iter, &mut content)?;

    println!("http method: {:?}", http_method);
    println!("path: {:?}", path);
    println!("content: {:#?}", content);

    Some((http_method, path, content))
}

fn parse_path_content<I>(input: &mut I, content: &mut PathContent) -> Option<()>
where
    I: Iterator<Item = TokenTree>,
{
    let mut content_iter = input
        .next()
        .and_then(|tt| match tt {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                Some(group.stream())
            }
            _ => None,
        })?
        .into_iter()
        .peekable();

    loop {
        let key = parse_key(&mut content_iter)?;
        println!("parsed key: {:?}", key);
        println!("next: {:?}", content_iter.peek());
        parse_delimiter(&mut content_iter, ':')?;

        match key.as_ref() {
            PathContentKeyRef::Ident("tags") => {
                content.tags = parse_group_list(&mut content_iter)?;
            }
            PathContentKeyRef::Ident("summary") => {
                content.summary = Some(parse_string(&mut content_iter)?);
            }
            PathContentKeyRef::Ident("description") => {
                content.description = Some(parse_string(&mut content_iter)?);
            }
            PathContentKeyRef::Ident("security") => {
                content.security = parse_group_list(&mut content_iter)?;
            }
            PathContentKeyRef::Ident("parameters") => {
                content.parameters = parse_path_parameters(&mut content_iter)?;
            }
            PathContentKeyRef::Ident("body") => {
                content.body = Some(parse_type_until(&mut content_iter, ',')?);
            }
            PathContentKeyRef::Code(code, _description) => {
                let response_model = parse_type_until(&mut content_iter, ',')?;
                content.responses.insert(code, response_model);
            }
            _ => return None,
        }

        if parse_trailing_delimiter(&mut content_iter, ',')? {
            break;
        }
    }

    Some(())
}

fn parse_key<I>(input: &mut Peekable<I>) -> Option<PathContentKey>
where
    I: Iterator<Item = TokenTree>,
    Peekable<I>: Clone,
{
    match input.peek()? {
        TokenTree::Ident(_) => parse_ident(input).map(PathContentKey::Ident),
        TokenTree::Literal(_) => {
            let code = parse_integer(input)?;
            let mut description = None;
            println!(
                "trying description: {:?}",
                input.clone().collect::<TokenStream>().to_string()
            );
            if let Some(TokenTree::Group(group)) = input.peek() {
                let mut desc_iter = group.stream().into_iter();
                description = Some(parse_string_or_ident(&mut desc_iter)?);
            }
            Some(PathContentKey::Code(code, description))
        }
        _ => None,
    }
}

fn parse_path_address<I>(
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
            _ => None,
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
            Some(token @ TokenTree::Literal(_)) => {
                println!("literal: {:?}", token);
                result.push(PathSegment::Path(parse_string(&mut address_iter)?));
            }
            Some(token @ TokenTree::Ident(_)) => {
                println!("ident: {:?}", token);
                let ident = {
                    let name = parse_ident(&mut address_iter)?;
                    name[..1].to_ascii_lowercase() + &name[1..]
                };
                result.push(PathSegment::Parameter(ident));
            }
            Some(token @ TokenTree::Group(_)) => {
                println!("group: {:?}", token);

                let (param_name, ty) = parse_path_address_named_segment(&mut address_iter)?;

                parameters.insert(
                    param_name.clone(),
                    Parameter {
                        description: None,
                        parameter_in: ParameterIn::Path,
                        required: true,
                        ty,
                    },
                );

                result.push(PathSegment::Parameter(param_name));
            }
            token => {
                println!("token: {:?}", token);
                return None;
            }
        };

        println!("iter: {:?}", result);

        if parse_trailing_delimiter(&mut address_iter, '/')? {
            break Some(result);
        }
    }
}

fn parse_path_address_named_segment<I>(input: &mut Peekable<I>) -> Option<(String, syn::TypePath)>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Group(group) => {
            let mut param_iter = group.stream().into_iter().peekable();
            let param_name = parse_ident(&mut param_iter)?;
            parse_delimiter(&mut param_iter, ':')?;
            let param_type = syn::parse::<syn::TypePath>(param_iter.collect()).ok()?;
            Some((param_name, param_type))
        }
        _ => None,
    })
}

fn parse_path_parameters<I>(input: &mut Peekable<I>) -> Option<HashMap<String, Parameter>>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
            let mut group_iter = group.stream().into_iter().peekable();

            let mut result = HashMap::new();

            loop {
                let (name, parameter) = parse_path_parameter_item(&mut group_iter)?;
                println!("parsed parameter: {:?}, {:?}", name, parameter);

                result.insert(name, parameter);

                if parse_trailing_delimiter(&mut group_iter, ',')? {
                    break Some(result);
                }
            }
        }
        _ => None,
    })
}

fn parse_path_parameter_item<I>(input: &mut I) -> Option<(String, Parameter)>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
            let mut group_iter = group.stream().into_iter().peekable();

            let parameter_in = parse_ident(&mut group_iter)
                .and_then(|ident| ParameterIn::from_str(&ident).ok())?;
            let name = parse_ident(&mut group_iter)?;
            parse_delimiter(&mut group_iter, ':')?;
            let ty = syn::parse::<syn::TypePath>(group_iter.collect()).ok()?;

            Some((
                name,
                Parameter {
                    description: None,
                    parameter_in,
                    required: parameter_in.required_by_default(),
                    ty,
                },
            ))
        }
        _ => None,
    })
}

fn parse_type_until<I>(input: &mut Peekable<I>, delimiter: char) -> Option<syn::TypePath>
where
    I: Iterator<Item = TokenTree>,
    Peekable<I>: Clone,
{
    let result = syn::parse::<syn::TypePath>(
        input
            .clone()
            .take_while(|_| match input.peek() {
                Some(TokenTree::Punct(punct)) if punct.as_char() == delimiter => false,
                item => {
                    println!("take while: {:?}", item);
                    input.next().is_some()
                }
            })
            .collect(),
    );
    result.ok()
}

fn parse_group_list<I>(input: &mut I) -> Option<Vec<String>>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Group(group)
            if matches!(group.delimiter(), Delimiter::Brace | Delimiter::Bracket) =>
        {
            let mut group_iter = group.stream().into_iter().peekable();
            let mut result = Vec::new();
            loop {
                result.push(parse_string_or_ident(&mut group_iter)?);
                if parse_trailing_delimiter(&mut group_iter, ',')? {
                    break Some(result);
                }
            }
        }
        token @ TokenTree::Ident(_) => syn::parse::<syn::Ident>(token.into())
            .map(|ident| vec![ident.to_string()])
            .ok(),
        token @ TokenTree::Literal(_) => syn::parse::<syn::LitStr>(token.into())
            .map(|literal| vec![literal.value()])
            .ok(),
        _ => None,
    })
}

fn parse_string_or_ident<I>(input: &mut I) -> Option<String>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        token @ TokenTree::Ident(_) => syn::parse::<syn::Ident>(token.into())
            .map(|ident| ident.to_string())
            .ok(),
        token @ TokenTree::Literal(_) => syn::parse::<syn::LitStr>(token.into())
            .map(|literal| literal.value())
            .ok(),
        _ => None,
    })
}

fn parse_string<I>(input: &mut I) -> Option<String>
where
    I: Iterator<Item = TokenTree>,
{
    input
        .next()
        .and_then(|token| syn::parse::<syn::LitStr>(token.into()).ok())
        .map(|literal: syn::LitStr| literal.value())
}

fn parse_ident<I>(input: &mut I) -> Option<String>
where
    I: Iterator<Item = TokenTree>,
{
    input
        .next()
        .and_then(|token| syn::parse::<syn::Ident>(token.into()).ok())
        .map(|ident| ident.to_string())
}

fn parse_integer<I, T>(input: &mut I) -> Option<T>
where
    I: Iterator<Item = TokenTree>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    input
        .next()
        .and_then(|token| syn::parse::<syn::LitInt>(token.into()).ok())
        .and_then(|literal| literal.base10_parse().ok())
}

fn parse_trailing_delimiter<I>(input: &mut Peekable<I>, delimiter: char) -> Option<bool>
where
    I: Iterator<Item = TokenTree>,
{
    if input.peek().is_some() {
        println!("input peek: {:?}", input.peek());
        parse_delimiter(input, delimiter)?;
    }
    Some(input.peek().is_none())
}

fn parse_delimiter<I>(input: &mut I, delimiter: char) -> Option<()>
where
    I: Iterator<Item = TokenTree>,
{
    input.next().and_then(|token| match token {
        TokenTree::Punct(punct) if punct.as_char() == delimiter => Some(()),
        _ => None,
    })
}

#[derive(Debug)]
enum PathContentKey {
    Ident(String),
    Code(u16, Option<String>),
}

impl PathContentKey {
    fn as_ref(&self) -> PathContentKeyRef {
        match self {
            Self::Ident(ident) => PathContentKeyRef::Ident(ident),
            Self::Code(code, description) => {
                PathContentKeyRef::Code(*code, description.as_ref().map(String::as_str))
            }
        }
    }
}

enum PathContentKeyRef<'a> {
    Ident(&'a str),
    Code(u16, Option<&'a str>),
}

#[derive(Default)]
struct PathContent {
    tags: Vec<String>,
    summary: Option<String>,
    description: Option<String>,
    security: Vec<String>,
    parameters: HashMap<String, Parameter>,
    body: Option<syn::TypePath>,
    responses: HashMap<u16, syn::TypePath>,
}

impl std::fmt::Debug for PathContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct ResponsesHelper<'a>(&'a HashMap<u16, syn::TypePath>);

        impl<'a> std::fmt::Debug for ResponsesHelper<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut fmt = f.debug_map();
                for (key, value) in self.0.iter() {
                    fmt.key(key).value(&value.to_token_stream().to_string());
                }
                fmt.finish()
            }
        }

        f.debug_struct("PathContent")
            .field("tags", &self.tags)
            .field("summary", &self.summary)
            .field("description", &self.description)
            .field("security", &self.security)
            .field("parameters", &self.parameters)
            .field("body", &self.body.to_token_stream().to_string())
            .field("responses", &ResponsesHelper(&self.responses))
            .finish()
    }
}

#[derive(Debug, Clone)]
enum PathSegment {
    Path(String),
    Parameter(String),
}

#[derive(Clone)]
struct Parameter {
    description: Option<String>,
    parameter_in: ParameterIn,
    required: bool,
    ty: syn::TypePath,
}

impl std::fmt::Debug for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parameter")
            .field("description", &self.description)
            .field("parameter_in", &self.parameter_in)
            .field("required", &self.required)
            .field("ty", &self.ty.to_token_stream().to_string())
            .finish()
    }
}

#[derive(Debug, Copy, Clone)]
enum ParameterIn {
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

#[derive(Debug, Copy, Clone)]
enum HttpMethod {
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
