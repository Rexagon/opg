use std::borrow::Cow;
use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::Write;

use either::*;
use serde::ser::{SerializeMap, SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};

/// OpenAPI Object
///
/// [specification](https://swagger.io/specification/#openapi-object)
#[derive(Debug, Clone, Default, Serialize)]
pub struct Opg {
    /// Semantic version number of the OpenAPI Specification version
    pub openapi: OpenApiVersion,

    /// Provides metadata about the API
    pub info: Info,

    /// A list of tags used by the specification with additional metadata
    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_tags"
    )]
    pub tags: BTreeMap<String, Tag>,

    /// An array of Server Objects, which provide connectivity information to a target server
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<Server>,

    /// The available paths and operations for the API
    #[serde(
        serialize_with = "serialize_ordered_entries",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub paths: Vec<(Path, PathValue)>,

    /// An element to hold various schemas for the specification
    pub components: Components,
}

/// OpenAPI version.
/// 3.0.3 by default
#[derive(Debug, Clone, Serialize)]
pub struct OpenApiVersion(String);

impl Default for OpenApiVersion {
    fn default() -> Self {
        Self(crate::OPENAPI_VERSION.to_owned())
    }
}

/// Serialize slice of tuples as map
fn serialize_ordered_entries<S, T1, T2>(
    entries: &[(T1, T2)],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T1: Serialize,
    T2: Serialize,
    S: Serializer,
{
    let mut ser = serializer.serialize_map(Some(entries.len()))?;

    entries
        .iter()
        .try_for_each(|(key, value)| ser.serialize_entry(key, value))?;

    ser.end()
}

/// Serialize map of tags as sequence
fn serialize_tags<S>(tags: &BTreeMap<String, Tag>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
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

/// Info Object
///
/// [specification](https://swagger.io/specification/#info-object)
#[derive(Debug, Clone, Default, Serialize)]
pub struct Info {
    /// The title of the API
    pub title: String,

    /// A short description of the API
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The version of the OpenAPI document
    pub version: String,
}

/// Tag Object
///
/// [specification](https://swagger.io/specification/#tag-object)
#[derive(Debug, Clone, Default, Serialize)]
pub struct Tag {
    /// A short description for the tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Server Object
///
/// [specification](https://swagger.io/specification/#server-object)
///
/// TODO: add variables section
#[derive(Debug, Clone, Serialize)]
pub struct Server {
    /// A URL to the target host
    pub url: String,

    /// An optional string describing the host designated by the URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Paths Object key
///
/// [specification](https://swagger.io/specification/#paths-object)
#[derive(Debug, Clone, Serialize)]
pub struct Path(#[serde(serialize_with = "serialize_path_elements")] pub Vec<PathElement>);

/// Serialize sequence of path elements as single string
fn serialize_path_elements<S>(elements: &[PathElement], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut result = String::new();

    for element in elements.iter().map(|element| match element {
        PathElement::Path(path) => Either::Left(path),
        PathElement::Parameter(param) => Either::Right(format!("{{{}}}", param)),
    }) {
        write!(&mut result, "/{}", element).unwrap();
    }

    serializer.serialize_str(&result)
}

/// Paths Object key part.
///
/// Describes path part between two '/'.
///
/// For example path `/pets/{petId}` is represented by array of values:
/// `PathElement::Path("pets"), PathElement::Parameter("petId")`
///
/// [specification](https://swagger.io/specification/#paths-object)
#[derive(Debug, Clone)]
pub enum PathElement {
    Path(String),
    Parameter(String),
}

/// Path Item Object
///
/// [specification](https://swagger.io/specification/#path-item-object)
#[derive(Debug, Clone, Default, Serialize)]
pub struct PathValue {
    /// An optional, string summary, intended to apply to all operations in this path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// An optional, string description, intended to apply to all operations in this path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A definitions of operations on this path.
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub operations: BTreeMap<HttpMethod, Operation>,

    /// A list of parameters that are applicable for all the operations described under this path
    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_parameters"
    )]
    pub parameters: BTreeMap<String, OperationParameter>,
}

/// Path Item Object operation type
///
/// [specification](https://swagger.io/specification/#path-item-object)
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
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

impl HttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "get",
            HttpMethod::PUT => "put",
            HttpMethod::POST => "post",
            HttpMethod::DELETE => "delete",
            HttpMethod::OPTIONS => "options",
            HttpMethod::HEAD => "head",
            HttpMethod::PATCH => "patch",
            HttpMethod::TRACE => "trace",
        }
    }
}

impl Serialize for HttpMethod {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

/// Operation Object
///
/// [specification](https://swagger.io/specification/#operation-object)
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// A list of tags for API documentation control
    ///
    /// Tags can be used for logical grouping of operations by resources or any other qualifier.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// A short summary of what the operation does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Unique string used to identify the operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,

    /// A verbose explanation of the operation behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Declares this operation to be deprecated
    #[serde(skip_serializing_if = "is_false")]
    pub deprecated: bool,

    /// A declaration of which security mechanisms can be used for this operation
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<BTreeMap<String, Vec<String>>>,

    /// The request body applicable for this operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_body: Option<RequestBody>,

    /// The list of possible responses as they are returned from executing this operation
    pub responses: BTreeMap<u16, Response>,

    /// A list of parameters that are applicable for this operation
    #[serde(
        skip_serializing_if = "BTreeMap::is_empty",
        serialize_with = "serialize_parameters"
    )]
    pub parameters: BTreeMap<String, OperationParameter>,

    /// A map of possible out-of band callbacks related to this operation
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub callbacks: BTreeMap<String, CallbackObject>,
}

impl Operation {
    pub fn with_summary<T>(&mut self, summary: &T) -> &mut Self
    where
        T: ToString + ?Sized,
    {
        self.summary = Some(summary.to_string());
        self
    }

    pub fn with_operation_id<T>(&mut self, operation_id: &T) -> &mut Self
    where
        T: ToString + ?Sized,
    {
        self.operation_id = Some(operation_id.to_string());
        self
    }

    pub fn with_description<T>(&mut self, description: &T) -> &mut Self
    where
        T: ToString + ?Sized,
    {
        self.description = Some(description.to_string());
        self
    }

    pub fn mark_deprecated(&mut self, deprecated: bool) -> &mut Self {
        self.deprecated = deprecated;
        self
    }

    pub fn with_request_body(&mut self, body: RequestBody) -> &mut Self {
        self.request_body = Some(body);
        self
    }
}

/// Request Body Object
///
/// [specification](https://swagger.io/specification/#request-body-object)
#[derive(Debug, Clone)]
pub struct RequestBody {
    /// A brief description of the request body
    pub description: Option<String>,

    /// Determines if the request body is required in the request. Defaults to true.
    pub required: bool,

    /// The content of the request body
    pub schema: ModelReference,
}

impl Serialize for RequestBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct RequestBodyHelper<'a> {
            #[serde(skip_serializing_if = "is_false")]
            required: bool,
            description: &'a Option<String>,
            content: ResponseContent<'a>,
        }

        RequestBodyHelper {
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

/// Response Object
///
/// [specification](https://swagger.io/specification/#response-object)
#[derive(Debug, Clone)]
pub struct Response {
    /// A short description of the response
    pub description: String,

    /// Response schema
    pub schema: Option<ModelReference>,
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct ResponseHelper<'a> {
            description: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<ResponseContent<'a>>,
        }

        ResponseHelper {
            description: &self.description,
            content: self.schema.as_ref().map(|schema| ResponseContent {
                media_type: ResponseMediaType { schema },
            }),
        }
        .serialize(serializer)
    }
}

/// helper for serde
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !*value
}

/// Response media type.
///
/// Currently behaves as a stub with value 'application/json'
#[derive(Serialize)]
struct ResponseMediaType<'a> {
    schema: &'a ModelReference,
}

/// Callback Object
///
/// [specification](https://swagger.io/specification/#callback-object)
#[derive(Debug, Clone, Default, Serialize)]
pub struct CallbackObject {
    /// The available paths and operations for the API
    #[serde(
        flatten,
        serialize_with = "serialize_ordered_entries",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub paths: Vec<(Path, PathValue)>,
}

/// Content Object
#[derive(Serialize)]
struct ResponseContent<'a> {
    #[serde(rename = "application/json")]
    media_type: ResponseMediaType<'a>,
}

/// Serialize map of parameters as sequence
fn serialize_parameters<S>(
    parameters: &BTreeMap<String, OperationParameter>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct OperationParameterHelper<'a> {
        name: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: &'a Option<String>,
        #[serde(rename = "in")]
        parameter_in: ParameterIn,
        #[serde(skip_serializing_if = "is_false")]
        required: bool,
        #[serde(skip_serializing_if = "is_false")]
        deprecated: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        schema: &'a Option<ModelReference>,
    }

    let mut ser = serializer.serialize_seq(Some(parameters.len()))?;

    parameters.iter().try_for_each(|(name, operation)| {
        ser.serialize_element(&OperationParameterHelper {
            name,
            description: &operation.description,
            parameter_in: operation.parameter_in,
            required: operation.required,
            deprecated: operation.deprecated,
            schema: &operation.schema,
        })
    })?;

    ser.end()
}

/// Parameter Object
///
/// [specification](https://swagger.io/specification/#parameter-object)
#[derive(Debug, Clone)]
pub struct OperationParameter {
    /// A brief description of the parameter
    pub description: Option<String>,

    /// The location of the parameter
    pub parameter_in: ParameterIn,

    /// Determines whether this parameter is mandatory
    ///
    /// If the parameter location is "path", this property is REQUIRED and its value MUST be true.
    /// Otherwise, the property MAY be included and its default value is false.
    pub required: bool,

    /// Declares this parameter to be deprecated
    pub deprecated: bool,

    /// The schema defining the type used for the parameter
    pub schema: Option<ModelReference>,
}

/// The location of the parameter
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}

/// Components Object
///
/// [specification](https://swagger.io/specification/#components-object)
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    /// An object to hold reusable Schema Objects
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub schemas: BTreeMap<String, Model>,

    /// An object to hold reusable Security Scheme Objects
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub security_schemes: BTreeMap<String, SecurityScheme>,
}

impl Components {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check name occurrence in schemas
    #[inline]
    pub fn contains_model(&self, name: &str) -> bool {
        self.schemas.contains_key(name)
    }

    /// Insert model into schema and return it's reference
    pub fn mention_schema<M>(&mut self, inline: bool, params: &ContextParams) -> ModelReference
    where
        M: OpgModel + ?Sized,
    {
        let reference = M::select_reference(self, inline, params);
        if let ModelReference::Link(link) = &reference {
            if !self.schemas.contains_key(link) {
                let structure = M::get_schema(self);
                self.schemas.insert(link.to_owned(), structure);
            }
        }
        reference
    }

    /// Insert security scheme and return it's name
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

    /// Verify schemas references
    #[allow(dead_code)]
    pub fn verify_schemas(&self) -> Result<(), String> {
        let cx = TraverseContext(&self.schemas);

        self.schemas
            .iter()
            .try_for_each(|(_, model)| model.traverse(cx))
            .map_err(|first_occurrence| first_occurrence.to_owned())
    }

    /// Manually insert model with specified name
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

/// Trait for schema objects generation
pub trait OpgModel {
    /// Get schema for this type
    fn get_schema(cx: &mut Components) -> Model;

    /// Get name of this type
    fn type_name() -> Option<Cow<'static, str>>;

    /// Get schema for this type with context parameters applied
    fn get_schema_with_params(cx: &mut Components, params: &ContextParams) -> Model {
        Self::get_schema(cx).apply_params(params)
    }

    /// Get link or inlined schema with context parameters applied
    #[inline]
    fn select_reference(
        cx: &mut Components,
        inline: bool,
        params: &ContextParams,
    ) -> ModelReference {
        match Self::type_name() {
            Some(link) if !inline => ModelReference::Link(link.into_owned()),
            _ => ModelReference::Inline(Self::get_schema(cx).apply_params(params)),
        }
    }
}

/// Context parameters
#[derive(Default)]
pub struct ContextParams {
    /// Brief description of this object inplace
    pub description: Option<String>,

    /// A true value adds "null" to the allowed type specified by the type keyword, only if type is
    /// explicitly defined within the same Schema Object
    pub nullable: Option<bool>,

    /// Possible enum variants of this string inplace
    pub variants: Option<Vec<String>>,

    /// Data type format inplace
    pub format: Option<String>,

    /// Example for this object inplace
    pub example: Option<String>,
}

/// Schema Object
///
/// [specification](https://swagger.io/specification/#schema-object)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    /// Brief description of this object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Type specific data
    #[serde(flatten)]
    pub data: ModelData,
}

impl Model {
    /// Apply context params for this object
    #[inline]
    pub fn apply_params(mut self, params: &ContextParams) -> Self {
        if let Some(description) = &params.description {
            self.description = Some(description.clone());
        }
        self.data = self.data.apply_params(params);
        self
    }

    /// Try merge other model into this object
    pub fn try_merge(&mut self, other: Model) -> Result<(), ModelMergeError> {
        match &mut self.data {
            ModelData::Single(ModelType {
                type_description: ModelTypeDescription::Object(self_object),
                ..
            }) => match other.data {
                ModelData::Single(ModelType {
                    type_description: ModelTypeDescription::Object(other_object),
                    ..
                }) => self_object.merge(other_object),
                _ => Err(ModelMergeError),
            },
            _ => Err(ModelMergeError),
        }
    }

    /// Check links
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.data.traverse(cx)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ModelMergeError;

/// Schema object representation
#[derive(Debug, Clone, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ModelData {
    Single(ModelType),
    OneOf(ModelOneOf),
    AllOf(ModelAllOf),
    AnyOf(ModelAnyOf),
}

impl ModelData {
    /// Apply context params for this object
    #[inline]
    pub fn apply_params(self, params: &ContextParams) -> Self {
        match self {
            ModelData::Single(data) => ModelData::Single(data.apply_params(params)),
            data => data, // TODO: apply params to oneOf, anyOf, and allOf
        }
    }

    /// Check links
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        match self {
            ModelData::Single(single) => single.traverse(cx),
            ModelData::OneOf(one_of) => one_of.traverse(cx),
            ModelData::AllOf(all_of) => all_of.traverse(cx),
            ModelData::AnyOf(any_of) => any_of.traverse(cx),
        }
    }
}

/// oneOf
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOneOf {
    pub one_of: Vec<ModelReference>,
}

impl ModelOneOf {
    /// Check links
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.one_of.iter().try_for_each(|item| item.traverse(cx))
    }
}

impl From<ModelOneOf> for ModelData {
    fn from(data: ModelOneOf) -> Self {
        ModelData::OneOf(data)
    }
}

/// allOf
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAllOf {
    pub all_of: Vec<ModelReference>,
}

impl ModelAllOf {
    /// Check links
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.all_of.iter().try_for_each(|item| item.traverse(cx))
    }
}

impl From<ModelAllOf> for ModelData {
    fn from(data: ModelAllOf) -> Self {
        ModelData::AllOf(data)
    }
}

/// anyOf
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAnyOf {
    pub any_of: Vec<ModelReference>,
}

impl ModelAnyOf {
    /// Check links
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.any_of.iter().try_for_each(|item| item.traverse(cx))
    }
}

impl From<ModelAnyOf> for ModelData {
    fn from(data: ModelAnyOf) -> Self {
        ModelData::AnyOf(data)
    }
}

/// type
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelType {
    /// Whether this type can have `null` value
    #[serde(skip_serializing_if = "is_false")]
    pub nullable: bool,

    /// Type description
    #[serde(flatten)]
    pub type_description: ModelTypeDescription,
}

impl ModelType {
    /// Apply context params for this object
    #[inline]
    pub fn apply_params(mut self, params: &ContextParams) -> Self {
        if let Some(nullable) = params.nullable {
            self.nullable = nullable;
        }
        self.type_description = self.type_description.apply_params(params);
        self
    }

    /// Check links
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

/// Data Type
///
/// [specification](https://swagger.io/specification/#data-types)
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
    /// Apply context params for this object
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

/// String data type
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelString {
    /// Possible values
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<String>>,

    /// Other type description data
    #[serde(flatten)]
    pub data: ModelSimple,
}

impl ModelString {
    /// Apply context params for this object
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

/// Simple model type description
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSimple {
    /// Value format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Example value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

impl ModelSimple {
    /// Apply context params for this object
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

/// Array type description
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelArray {
    pub items: Box<ModelReference>,
}

impl ModelArray {
    /// Check links
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.items.traverse(cx)
    }
}

impl From<ModelArray> for ModelTypeDescription {
    fn from(data: ModelArray) -> Self {
        ModelTypeDescription::Array(data)
    }
}

/// Object type description
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelObject {
    /// Object properties
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, ModelReference>,

    /// Additional properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<ModelReference>>,

    /// Required properties
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

impl ModelObject {
    /// Manually add property
    pub fn add_property(
        &mut self,
        property: String,
        property_type: ModelReference,
        is_required: bool,
    ) -> Result<(), ModelMergeError> {
        let entry = match self.properties.entry(property.clone()) {
            Entry::Vacant(entry) => entry,
            _ => return Err(ModelMergeError),
        };

        entry.insert(property_type);

        if is_required {
            self.required.push(property);
        }

        Ok(())
    }

    /// Merge other object into self
    pub fn merge(&mut self, another: ModelObject) -> Result<(), ModelMergeError> {
        another
            .properties
            .into_iter()
            .try_for_each(
                |(property, property_model)| match self.properties.entry(property) {
                    Entry::Vacant(entry) => {
                        entry.insert(property_model);
                        Ok(())
                    }
                    _ => Err(ModelMergeError),
                },
            )?;

        another
            .required
            .into_iter()
            .for_each(|property| self.required.push(property));

        Ok(())
    }

    /// Check links
    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        self.properties
            .iter()
            .map(|(_, reference)| reference)
            .chain(self.additional_properties.iter().map(|item| item.as_ref()))
            .try_for_each(|reference| reference.traverse(cx))
    }
}

impl From<ModelObject> for ModelTypeDescription {
    fn from(data: ModelObject) -> Self {
        ModelTypeDescription::Object(data)
    }
}

/// Model reference
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ModelReference {
    /// `$ref: "#/components/schemas/..."`
    #[serde(serialize_with = "serialize_model_reference_link")]
    Link(String),

    /// Inlined model
    Inline(Model),

    /// Any type, `{}`
    #[serde(serialize_with = "serialize_model_reference_any")]
    Any,
}

/// Serialize link as struct with `$ref` field
fn serialize_model_reference_link<S, N>(name: &N, serializer: S) -> Result<S::Ok, S::Error>
where
    N: std::fmt::Display,
    S: Serializer,
{
    let mut ser = serializer.serialize_map(Some(1))?;
    ser.serialize_entry(
        "$ref",
        &format!("{}{}", crate::SCHEMA_REFERENCE_PREFIX, name),
    )?;
    ser.end()
}

/// Serialize any field as `{}`
fn serialize_model_reference_any<S>(serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_struct("Any", 0)?.end()
}

impl ModelReference {
    /// Check links
    fn traverse<'a>(&'a self, mut cx: TraverseContext<'a>) -> Result<(), &'a str> {
        match &self {
            ModelReference::Link(ref link) => cx.check(link),
            _ => Ok(()),
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

/// Security Scheme Object
///
/// [specification](https://swagger.io/specification/#security-scheme-object)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SecurityScheme {
    Http(HttpSecurityScheme),
    ApiKey(ApiKeySecurityScheme),
    // TODO: add `oath2` and `openIdConnect`
}

/// HTTP security scheme
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "scheme")]
pub enum HttpSecurityScheme {
    Basic {
        /// A short description for security scheme
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Bearer {
        #[serde(rename = "bearerFormat", skip_serializing_if = "Option::is_none")]
        format: Option<String>,

        /// A short description for security scheme
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
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

/// Api key security scheme
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecurityScheme {
    /// The location of the API key
    #[serde(rename = "in")]
    pub parameter_in: ParameterIn,

    /// The name of the header, query or cookie parameter to be used
    pub name: String,

    /// A short description for security scheme
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl From<ApiKeySecurityScheme> for SecurityScheme {
    fn from(data: ApiKeySecurityScheme) -> Self {
        SecurityScheme::ApiKey(data)
    }
}

/// Stub for macros
pub struct ParameterNotSpecified;
