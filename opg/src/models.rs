use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::Write;

use either::*;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Default, Serialize)]
pub struct Opg {
    pub openapi: OpgOpenApi,
    pub info: OpgInfo,

    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_tags"
    )]
    pub tags: BTreeMap<String, OpgTag>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<OpgServer>,

    #[serde(
        serialize_with = "serialize_ordered_entries",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub paths: Vec<(OpgPath, OpgPathValue)>,

    pub components: OpgComponents,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpgOpenApi(String);

impl Default for OpgOpenApi {
    fn default() -> Self {
        Self(crate::OPENAPI_VERSION.to_owned())
    }
}

fn serialize_ordered_entries<S, T1, T2>(
    entries: &[(T1, T2)],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T1: Serialize,
    T2: Serialize,
    S: serde::ser::Serializer,
{
    let mut ser = serializer.serialize_map(Some(entries.len()))?;

    entries
        .iter()
        .try_for_each(|(key, value)| ser.serialize_entry(key, value))?;

    ser.end()
}

fn serialize_tags<S>(tags: &BTreeMap<String, OpgTag>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    #[derive(Serialize)]
    pub struct OpgTagHelper<'a> {
        name: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: &'a Option<String>,
    }

    let mut ser = serializer.serialize_seq(Some(tags.len()))?;

    tags.iter().try_for_each(|(name, tag)| {
        ser.serialize_element(&OpgTagHelper {
            name,
            description: &tag.description,
        })
    })?;

    ser.end()
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct OpgInfo {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct OpgTag {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpgServer {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpgPath(#[serde(serialize_with = "serialize_path_elements")] pub Vec<OpgPathElement>);

fn serialize_path_elements<S>(elements: &[OpgPathElement], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    let mut result = String::new();

    for element in elements.iter().map(|element| match element {
        OpgPathElement::Path(path) => Either::Left(path),
        OpgPathElement::Parameter(param) => Either::Right(format!("{{{}}}", param)),
    }) {
        write!(&mut result, "/{}", element).unwrap();
    }

    serializer.serialize_str(&result)
}

#[derive(Debug, Clone)]
pub enum OpgPathElement {
    Path(String),
    Parameter(String),
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct OpgPathValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(
        flatten,
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_operations"
    )]
    pub operations: BTreeMap<OpgHttpMethod, OpgOperation>,
    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_parameters"
    )]
    pub parameters: BTreeMap<String, OpgOperationParameter>,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum OpgHttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

impl OpgHttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            OpgHttpMethod::GET => "get",
            OpgHttpMethod::POST => "post",
            OpgHttpMethod::PUT => "put",
            OpgHttpMethod::DELETE => "delete",
        }
    }
}

fn serialize_operations<S>(
    operations: &BTreeMap<OpgHttpMethod, OpgOperation>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    let mut ser = serializer.serialize_map(Some(operations.len()))?;

    operations.iter().try_for_each(|(name, operation)| {
        ser.serialize_entry(&name.as_str().to_ascii_lowercase(), operation)
    })?;

    ser.end()
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpgOperation {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<BTreeMap<String, Vec<String>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<OpgRequestBody>,

    pub responses: BTreeMap<u16, OpgResponse>,

    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_parameters"
    )]
    pub parameters: BTreeMap<String, OpgOperationParameter>,
}

#[derive(Debug, Clone)]
pub struct OpgRequestBody {
    pub description: Option<String>,
    pub required: bool,
    pub schema: ModelReference,
}

impl Serialize for OpgRequestBody {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct OpgRequestBodyHelper<'a> {
            #[serde(skip_serializing_if = "is_false")]
            required: bool,
            description: &'a Option<String>,
            content: ResponseContent<'a>,
        }

        OpgRequestBodyHelper {
            required: self.required,
            description: &self.description,
            content: ResponseContent {
                media_type: ResponseMediaType {
                    schema: &self.schema,
                },
            },
        }
        .serialize(serializer)
    }
}

#[derive(Debug, Clone)]
pub struct OpgResponse {
    pub description: String,
    pub schema: ModelReference,
}

impl Serialize for OpgResponse {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct OpgResponseHelper<'a> {
            description: &'a str,
            content: ResponseContent<'a>,
        }

        OpgResponseHelper {
            description: &self.description,
            content: ResponseContent {
                media_type: ResponseMediaType {
                    schema: &self.schema,
                },
            },
        }
        .serialize(serializer)
    }
}

// helper for serde
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Serialize)]
struct ResponseMediaType<'a> {
    schema: &'a ModelReference,
}

#[derive(Serialize)]
struct ResponseContent<'a> {
    #[serde(rename = "application/json")]
    media_type: ResponseMediaType<'a>,
}

fn serialize_parameters<S>(
    parameters: &BTreeMap<String, OpgOperationParameter>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct OpgOperationParameterHelper<'a> {
        name: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: &'a Option<String>,
        #[serde(rename = "in")]
        parameter_in: ParameterIn,
        #[serde(skip_serializing_if = "is_false")]
        required: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: &'a Option<ModelReference>,
    }

    let mut ser = serializer.serialize_seq(Some(parameters.len()))?;

    parameters.iter().try_for_each(|(name, operation)| {
        ser.serialize_element(&OpgOperationParameterHelper {
            name,
            description: &operation.description,
            parameter_in: operation.parameter_in,
            required: operation.required,
            schema: &operation.schema,
        })
    })?;

    ser.end()
}

#[derive(Debug, Clone)]
pub struct OpgOperationParameter {
    pub description: Option<String>,
    pub parameter_in: ParameterIn,
    pub required: bool,
    pub schema: Option<ModelReference>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpgComponents {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub schemas: BTreeMap<String, Model>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub security_schemes: BTreeMap<String, SecurityScheme>,
}

impl OpgComponents {
    pub fn new() -> Self {
        Self {
            schemas: BTreeMap::new(),
            security_schemes: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn contains_model(&self, name: &str) -> bool {
        self.schemas.contains_key(name)
    }

    pub fn mention_schema<M>(&mut self, inline: bool, params: &ContextParams) -> ModelReference
    where
        M: OpgModel,
    {
        let reference = M::select_reference(self, inline, &params);
        if let ModelReference::Link(link) = &reference {
            if !self.schemas.contains_key(link) {
                let structure = M::get_schema(self);
                self.schemas.insert(link.to_owned(), structure);
            }
        }
        reference
    }

    pub fn mention_security_scheme<T>(&mut self, name: String, security_scheme: &T) -> String
    where
        T: Clone,
        SecurityScheme: From<T>,
    {
        if !self.security_schemes.contains_key(&name) {
            self.security_schemes
                .insert(name.clone(), security_scheme.clone().into());
        }
        name
    }

    #[allow(dead_code)]
    pub fn verify_schemas(&self) -> Result<(), String> {
        let cx = TraverseContext(&self.schemas);

        self.schemas
            .iter()
            .try_for_each(|(_, model)| model.traverse(cx))
            .map_err(|first_occurrence| first_occurrence.to_owned())
    }

    pub fn add_model<N>(&mut self, name: N, model: Model)
    where
        N: ToString,
    {
        if let std::collections::btree_map::Entry::Vacant(entry) =
            self.schemas.entry(name.to_string())
        {
            entry.insert(model);
        }
    }
}

pub trait OpgModel {
    fn get_schema(cx: &mut OpgComponents) -> Model;

    fn get_type_name() -> Option<&'static str>;

    fn get_schema_with_params(cx: &mut OpgComponents, params: &ContextParams) -> Model {
        Self::get_schema(cx).apply_params(params)
    }

    #[inline]
    fn select_reference(
        cx: &mut OpgComponents,
        inline: bool,
        params: &ContextParams,
    ) -> ModelReference {
        match Self::get_type_name() {
            Some(link) if !inline => ModelReference::Link(link.to_owned()),
            _ => ModelReference::Inline(Self::get_schema(cx).apply_params(params)),
        }
    }
}

#[derive(Default)]
pub struct ContextParams {
    pub description: Option<String>,
    pub nullable: Option<bool>,
    pub variants: Option<Vec<String>>,
    pub format: Option<String>,
    pub example: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub data: ModelData,
}

impl Model {
    #[inline]
    pub fn apply_params(mut self, params: &ContextParams) -> Self {
        if let Some(description) = &params.description {
            self.description = Some(description.clone());
        }
        self.data = self.data.apply_params(params);
        self
    }

    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.data.traverse(cx)
    }

    pub fn try_merge(&mut self, other: Model) -> Result<(), ()> {
        match &mut self.data {
            ModelData::Single(ModelType {
                type_description: ModelTypeDescription::Object(self_object),
                ..
            }) => match other.data {
                ModelData::Single(ModelType {
                    type_description: ModelTypeDescription::Object(other_object),
                    ..
                }) => self_object.merge(other_object),
                _ => Err(()),
            },
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ModelData {
    Single(ModelType),
    OneOf(ModelOneOf),
}

impl ModelData {
    #[inline]
    pub fn apply_params(self, params: &ContextParams) -> Self {
        match self {
            ModelData::Single(data) => ModelData::Single(data.apply_params(params)),
            one_of => one_of, // TODO: apply params to oneOf
        }
    }

    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        match self {
            ModelData::Single(single) => single.traverse(cx),
            ModelData::OneOf(one_of) => one_of.traverse(cx),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOneOf {
    pub one_of: Vec<ModelReference>,
}

impl ModelOneOf {
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.one_of.iter().try_for_each(|item| item.traverse(cx))
    }
}

impl From<ModelOneOf> for ModelData {
    fn from(data: ModelOneOf) -> Self {
        ModelData::OneOf(data)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelType {
    #[serde(skip_serializing_if = "is_false")]
    pub nullable: bool,
    #[serde(flatten)]
    pub type_description: ModelTypeDescription,
}

impl ModelType {
    #[inline]
    pub fn apply_params(mut self, params: &ContextParams) -> Self {
        if let Some(nullable) = params.nullable {
            self.nullable = nullable;
        }
        self.type_description = self.type_description.apply_params(params);
        self
    }

    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        match &self.type_description {
            ModelTypeDescription::Array(array) => array.traverse(cx),
            ModelTypeDescription::Object(object) => object.traverse(cx),
            _ => Ok(()),
        }
    }
}

impl From<ModelType> for ModelData {
    fn from(data: ModelType) -> Self {
        ModelData::Single(data)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ModelTypeDescription {
    String(ModelString),
    Number(ModelSimple),
    Integer(ModelSimple),
    Boolean,
    Array(ModelArray),
    Object(ModelObject),
}

impl ModelTypeDescription {
    #[inline]
    pub fn apply_params(self, params: &ContextParams) -> Self {
        match self {
            ModelTypeDescription::String(string) => {
                ModelTypeDescription::String(string.apply_params(params))
            }
            ModelTypeDescription::Number(number) => {
                ModelTypeDescription::Number(number.apply_params(params))
            }
            ModelTypeDescription::Integer(integer) => {
                ModelTypeDescription::Integer(integer.apply_params(params))
            }
            other => other,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelString {
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<String>>,
    #[serde(flatten)]
    pub data: ModelSimple,
}

impl ModelString {
    #[inline]
    pub fn apply_params(mut self, params: &ContextParams) -> Self {
        if let Some(variants) = &params.variants {
            self.variants = Some(variants.clone());
        }
        self.data = self.data.apply_params(params);
        self
    }
}

impl From<ModelString> for ModelTypeDescription {
    fn from(data: ModelString) -> Self {
        ModelTypeDescription::String(data)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSimple {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

impl ModelSimple {
    #[inline]
    pub fn apply_params(mut self, params: &ContextParams) -> Self {
        if let Some(format) = &params.format {
            self.format = Some(format.clone());
        }
        if let Some(example) = &params.example {
            self.example = Some(example.clone());
        }
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelArray {
    pub items: Box<ModelReference>,
}

impl ModelArray {
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.items.traverse(cx)
    }
}

impl From<ModelArray> for ModelTypeDescription {
    fn from(data: ModelArray) -> Self {
        ModelTypeDescription::Array(data)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelObject {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, ModelReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<ModelReference>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

impl ModelObject {
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.properties
            .iter()
            .map(|(_, reference)| reference)
            .chain(self.additional_properties.iter().map(|item| item.as_ref()))
            .try_for_each(|reference| reference.traverse(cx))
    }

    pub fn add_property(
        &mut self,
        property: String,
        property_type: ModelReference,
        is_required: bool,
    ) -> Result<(), ()> {
        let entry = match self.properties.entry(property.clone()) {
            Entry::Vacant(entry) => entry,
            _ => return Err(()),
        };

        entry.insert(property_type);

        if is_required {
            self.required.push(property);
        }

        Ok(())
    }

    pub fn merge(&mut self, another: ModelObject) -> Result<(), ()> {
        another
            .properties
            .into_iter()
            .try_for_each(
                |(property, property_model)| match self.properties.entry(property) {
                    Entry::Vacant(entry) => {
                        entry.insert(property_model);
                        Ok(())
                    }
                    _ => Err(()),
                },
            )?;

        another
            .required
            .into_iter()
            .for_each(|property| self.required.push(property));

        Ok(())
    }
}

impl From<ModelObject> for ModelTypeDescription {
    fn from(data: ModelObject) -> Self {
        ModelTypeDescription::Object(data)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ModelReference {
    #[serde(serialize_with = "serialize_model_reference_link")]
    Link(String),
    Inline(Model),
}

fn serialize_model_reference_link<S, N>(name: &N, serializer: S) -> Result<S::Ok, S::Error>
where
    N: std::fmt::Display,
    S: serde::ser::Serializer,
{
    let mut ser = serializer.serialize_map(Some(1))?;
    ser.serialize_entry(
        "$ref",
        &format!("{}{}", crate::SCHEMA_REFERENCE_PREFIX, name),
    )?;
    ser.end()
}

impl ModelReference {
    fn traverse<'a>(&'a self, mut cx: TraverseContext<'a>) -> Result<(), &'a str> {
        match &self {
            ModelReference::Link(ref link) => cx.check(link),
            ModelReference::Inline(_) => Ok(()),
        }
    }
}

#[derive(Copy, Clone)]
struct TraverseContext<'a>(&'a BTreeMap<String, Model>);

impl<'a> TraverseContext<'a> {
    fn check(&mut self, link: &'a str) -> Result<(), &'a str> {
        if self.0.contains_key(link) {
            Ok(())
        } else {
            Err(link)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SecurityScheme {
    Http(HttpSecurityScheme),
    ApiKey(ApiKeySecurityScheme),
    // TODO: add `oath2` and `openIdConnect`
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "scheme")]
pub enum HttpSecurityScheme {
    Basic,
    Bearer {
        #[serde(rename = "bearerFormat", skip_serializing_if = "Option::is_none")]
        format: Option<String>,
    },
}

impl From<HttpSecurityScheme> for SecurityScheme {
    fn from(data: HttpSecurityScheme) -> Self {
        SecurityScheme::Http(data)
    }
}

pub enum HttpSecuritySchemeKind {
    Basic,
    Bearer,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecurityScheme {
    #[serde(rename = "in")]
    pub parameter_in: ParameterIn,
    pub name: String,
}

impl From<ApiKeySecurityScheme> for SecurityScheme {
    fn from(data: ApiKeySecurityScheme) -> Self {
        SecurityScheme::ApiKey(data)
    }
}

pub struct ParameterNotSpecified;
