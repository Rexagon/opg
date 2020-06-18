use std::collections::{btree_map::Entry, BTreeMap};

use serde::export::fmt::Display;
use serde::Serialize;

pub struct OpgContext {
    models: BTreeMap<String, Model>,
}

impl OpgContext {
    pub fn new() -> Self {
        Self {
            models: BTreeMap::new(),
        }
    }

    #[inline(always)]
    pub fn contains_model(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    pub fn add_model<N>(&mut self, name: N, model: Model) -> Option<Model>
    where
        N: Display,
    {
        self.models.insert(prepare_model_reference(name), model)
    }

    pub fn verify_models(&self) -> Result<(), String> {
        let cx = TraverseContext(&self.models);

        self.models
            .iter()
            .try_for_each(|(_, model)| model.traverse(cx))
            .map_err(|first_occurrence| first_occurrence.to_owned())
    }
}

pub trait OpgModel {
    fn get_structure() -> Model;
    fn get_structure_with_params(params: &ContextParams) -> Model {
        Self::get_structure().apply_params(params)
    }
}

pub struct ContextParams {
    pub description: Option<String>,
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
    #[inline(always)]
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
            ModelData::Single(ModelTypeDescription::Object(self_object)) => match other.data {
                ModelData::Single(ModelTypeDescription::Object(other_object)) => {
                    self_object.merge(other_object)
                }
                _ => Err(()),
            },
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged, rename_all = "camelCase")]
pub enum ModelData {
    Single(ModelTypeDescription),
    OneOf(ModelOneOf),
}

impl ModelData {
    #[inline(always)]
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
    #[inline(always)]
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

    fn traverse<'a>(&'a self, cx: TraverseContext<'a>) -> Result<(), &'a str> {
        match self {
            ModelTypeDescription::Array(array) => array.traverse(cx),
            ModelTypeDescription::Object(object) => object.traverse(cx),
            _ => Ok(()),
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
    #[inline(always)]
    pub fn apply_params(mut self, params: &ContextParams) -> Self {
        if let Some(variants) = &params.variants {
            self.variants = Some(variants.clone());
        }
        self.data = self.data.apply_params(params);
        self
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
    #[inline(always)]
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

#[derive(Debug, Clone, Serialize)]
pub struct ModelReferenceLink {
    #[serde(rename = "$ref")]
    pub reference: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ModelReference {
    Link(ModelReferenceLink),
    Inline(Model),
}

impl ModelReference {
    fn traverse<'a>(&'a self, mut cx: TraverseContext<'a>) -> Result<(), &'a str> {
        match &self {
            ModelReference::Link(ModelReferenceLink { ref reference }) => cx.check(reference),
            ModelReference::Inline(_) => Ok(()),
        }
    }
}

#[derive(Copy, Clone)]
struct TraverseContext<'a>(&'a BTreeMap<String, Model>);

impl<'a> TraverseContext<'a> {
    fn check(&mut self, reference: &'a str) -> Result<(), &'a str> {
        if self.0.contains_key(reference) {
            Ok(())
        } else {
            Err(reference)
        }
    }
}

trait FromStrangeTuple<T> {
    fn extract(self) -> Option<T>;
}

impl<T> FromStrangeTuple<T> for () {
    fn extract(self) -> Option<T> {
        None
    }
}

impl<T> FromStrangeTuple<T> for (T,) {
    fn extract(self) -> Option<T> {
        Some(self.0)
    }
}

fn prepare_model_reference<N>(name: N) -> String
where
    N: Display,
{
    format!("{}{}", SCHEMA_REFERENCE_PREFIX, name)
}

macro_rules! describe_type(
    (raw_model => $model:ident) => {
        $model
    };

    (raw_type => {
        $(description: $description:literal)?
        ident: $type:ident
    }) => {
        Model {
            description: ($($description.to_string(),)?).extract(),
            data: $type,
        }
    };

    (string => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
        $(variants: [$($variants:literal),*])?
    }) => {
        Model {
            description: ($($description.to_string(),)?).extract(),
            data: ModelData::Single(ModelTypeDescription::String(ModelString {
                variants: ($(vec![$($variants.to_string()),*],)?).extract(),
                data: ModelSimple {
                    format: ($($format.to_string(),)?).extract(),
                    example: ($($example.to_string(),)?).extract(),
                }
            }))
        }
    };

    (number => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
    }) => {
        Model {
            description: ($($description.to_string(),)?).extract(),
            data: ModelData::Single(ModelTypeDescription::Number(ModelSimple {
                format: ($($format.to_string(),)?).extract(),
                example: ($($example.to_string(),)?).extract(),
            }))
        }
    };

    (integer => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
    }) => {
        Model {
            description: ($($description.to_string(),)?).extract(),
            data: ModelData::Single(ModelTypeDescription::Integer(ModelSimple {
                format: ($($format.to_string(),)?).extract(),
                example: ($($example.to_string(),)?).extract(),
            }))
        }
    };

    (boolean => {
        $(description: $description:literal)?
    }) => {
        Model {
            description: ($($description.to_string(),)?).extract(),
            data: ModelData::Single(ModelTypeDescription::Boolean)
        }
    };

    (array => {
        $(description: $description:literal)?
        items: ($($property_tail:tt)*)
    }) => {
        Model {
            description: ($($description.to_string(),)?).extract(),
            data: ModelData::Single(ModelTypeDescription::Array(ModelArray {
                items: Box::new(describe_type!(@object_property_value $($property_tail)*))
            }))
        }
    };

    (object => {
        $(description: $description:literal)?
        properties: {
            $($property_name:ident$([$required:tt])?: ($($property_tail:tt)*))*
        }
    }) => {{
        let mut properties = BTreeMap::new();
        #[allow(unused_mut)]
        let mut required = Vec::new();

        $(describe_type!(@object_property [properties, required] $property_name$([$required])?: ($($property_tail)*)));*;

        Model {
            description: ($($description.to_string(),)?).extract(),
            data: ModelData::Single(ModelTypeDescription::Object(ModelObject {
                properties,
                additional_properties: Default::default(),
                required,
            }))
        }
    }};

    (@object_property [$properties:ident, $required:ident] $property_name:ident: ($($property_tail:tt)*)) => {
        $properties.insert(stringify!($property_name).to_string(), describe_type!(@object_property_value $($property_tail)*));
    };


    (@object_property [$properties:ident, $required:ident] $property_name:ident[required]: ($($property_tail:tt)*)) => {
        describe_type!(@object_property [$properties, $required] $property_name: ($($property_tail)*));
        $required.push(stringify!($property_name).to_owned());
    };

    (@object_property_value link => $ref:literal) => {
        ModelReference::Link(ModelReferenceLink {
            reference: format!("{}{}", SCHEMA_REFERENCE_PREFIX, $ref)
        })
    };

    (@object_property_value link => $ref:ident) => {
        ModelReference::Link(ModelReferenceLink {
            reference: format!("{}{}", SCHEMA_REFERENCE_PREFIX, $ref)
        })
    };

    (@object_property_value $type:ident => $($tail:tt)*) => {
        ModelReference::Inline(describe_type!($type => $($tail)*))
    }
);

macro_rules! impl_opg_model(
    ($type:ty => $serialized_type:ident) => {
        impl OpgModel for $type {
            fn get_structure() -> Model {
                describe_type!($serialized_type => {})
            }
        }
    };
);

impl_opg_model!(String => string);

impl_opg_model!(i8 => integer);
impl_opg_model!(u8 => integer);
impl_opg_model!(i16 => integer);
impl_opg_model!(u16 => integer);
impl_opg_model!(i32 => integer);
impl_opg_model!(u32 => integer);
impl_opg_model!(i64 => integer);
impl_opg_model!(u64 => integer);

impl_opg_model!(f32 => number);
impl_opg_model!(f64 => number);

impl_opg_model!(bool => boolean);

impl<T> OpgModel for Vec<T>
where
    T: OpgModel,
{
    fn get_structure() -> Model {
        Model {
            description: None,
            data: ModelData::Single(ModelTypeDescription::Array(ModelArray {
                items: Box::new(ModelReference::Inline(T::get_structure())),
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let model = Model {
            description: Some("Some type".to_owned()),
            data: ModelData::Single(ModelTypeDescription::Object(ModelObject {
                properties: {
                    let mut properties = BTreeMap::new();
                    properties.insert(
                        "id".to_owned(),
                        ModelReference::Link(ModelReferenceLink {
                            reference: "#/components/schemas/TransactionId".to_owned(),
                        }),
                    );
                    properties.insert(
                        "amount".to_owned(),
                        ModelReference::Inline(Model {
                            description: None,
                            data: ModelData::Single(ModelTypeDescription::String(ModelString {
                                variants: None,
                                data: ModelSimple {
                                    format: None,
                                    example: None,
                                },
                            })),
                        }),
                    );

                    properties
                },
                additional_properties: Default::default(),
                required: vec![
                    "id".to_owned(),
                    "amount".to_owned(),
                    "currency".to_owned(),
                    "paymentType".to_owned(),
                    "status".to_owned(),
                ],
            })),
        };

        assert_eq!(
            serde_yaml::to_string(&model).unwrap(),
            r##"---
description: Some type
type: object
properties:
  amount:
    type: string
  id:
    $ref: "#/components/schemas/TransactionId"
required:
  - id
  - amount
  - currency
  - paymentType
  - status"##
        );
    }

    #[test]
    fn test_macro() {
        let sub = describe_type!(string => {
            description: "Test"
        });

        let model = describe_type!(object => {
            description: "Hello world"
            properties: {
                id[required]: (link => "TransactionId")
                test[required]: (object => {
                    properties: {
                        sub: (link => "TransactionId")
                    }
                })
                test_object: (string => {
                    format: "uuid"
                    variants: ["aaa", "bbb"]
                })
                test_integer: (integer => {
                    format: "timestamp"
                    example: "1591956576404"
                })
                test_boolean: (boolean => {})
                test_array: (array => {
                    items: (string => {})
                })
                test_raw_model: (raw_model => sub)
            }
        });

        assert_eq!(
            serde_yaml::to_string(&model).unwrap(),
            r##"---
description: Hello world
type: object
properties:
  id:
    $ref: "#/components/schemas/TransactionId"
  test:
    type: object
    properties:
      sub:
        $ref: "#/components/schemas/TransactionId"
    required: []
  test_array:
    type: array
    items:
      type: string
  test_boolean:
    type: boolean
  test_integer:
    type: integer
    format: timestamp
    example: "1591956576404"
  test_object:
    type: string
    enum:
      - aaa
      - bbb
    format: uuid
  test_raw_model:
    description: Test
    type: string
required:
  - id
  - test"##
        );
    }

    #[test]
    fn test_valid_models_context() {
        let mut cx = OpgContext::new();

        cx.add_model(
            "TransactionId",
            describe_type!(string => {
                description: "Transaction UUID"
                format: "uuid"
                example: "000..000-000..000-00..00"
            }),
        );

        cx.add_model(
            "SomeResponse",
            describe_type!(object => {
                properties: {
                    id: (link => "TransactionId")
                }
            }),
        );

        assert_eq!(cx.verify_models(), Ok(()));
    }

    #[test]
    fn test_invalid_models_context() {
        let mut cx = OpgContext::new();

        let invalid_link = "TransactionId";

        cx.add_model(
            "SomeResponse",
            describe_type!(object => {
                properties: {
                    id: (link => invalid_link)
                }
            }),
        );

        assert_eq!(
            cx.verify_models(),
            Err(prepare_model_reference(invalid_link))
        );
    }
}

pub const SCHEMA_REFERENCE_PREFIX: &'static str = "#/components/schemas/";
