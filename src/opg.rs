use std::collections::{BTreeMap, HashSet};

use itertools::*;
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

    pub fn contains_model(&self, name: &str) -> bool {
        self.models.contains_key(name)
    }

    pub fn verify_models(&self) -> Result<(), Vec<&str>> {
        let invalid_links = self
            .models
            .iter()
            .flat_map(|(_, model)| match &model.data {
                ModelData::Single(single) => Either::Left(std::iter::once(single)),
                ModelData::OneOf(multiple) => Either::Right(multiple.one_of.iter()),
            })
            .filter_map(|type_description| match type_description {
                ModelTypeDescription::Array(ModelArray { items }) => match items.as_ref() {
                    ModelReference::Link(link) => {
                        Some(Either::Left(std::iter::once(link.reference.as_str())))
                    }
                    _ => None,
                },
                ModelTypeDescription::Object(object) => {
                    Some(Either::Right(object.properties.iter().filter_map(
                        |(_, property)| match property {
                            ModelReference::Link(link) => Some(link.reference.as_str()),
                            _ => None,
                        },
                    )))
                }
                _ => None,
            })
            .flatten()
            .fold(Vec::new(), |mut invalid_links, link| {
                if !self.models.contains_key(link) {
                    invalid_links.push(link);
                }
                invalid_links
            });

        if invalid_links.is_empty() {
            Ok(())
        } else {
            Err(invalid_links)
        }
    }
}

pub trait OpgModel {
    fn get_structure() -> Model;
}

#[derive(Debug, Clone, Serialize)]
pub struct Model {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub data: ModelData,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ModelData {
    Single(ModelTypeDescription),
    OneOf(ModelOneOf),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOneOf {
    pub one_of: Vec<ModelTypeDescription>,
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

#[derive(Debug, Clone, Serialize)]
pub struct ModelString {
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub variants: Option<Vec<String>>,
    #[serde(flatten)]
    pub data: ModelSimple,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelSimple {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelArray {
    pub items: Box<ModelReference>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelObject {
    pub properties: BTreeMap<String, ModelReference>,
    pub required: Vec<String>,
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
        let mut required = Vec::new();

        $(describe_type!(@object_property [properties, required] $property_name$([$required])?: ($($property_tail)*)));*;

        Model {
            description: ($($description.to_string(),)?).extract(),
            data: ModelData::Single(ModelTypeDescription::Object(ModelObject {
                properties,
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

    (@object_property_value link => $ref:ty) => {
        ModelReference::Link(ModelReferenceLink {
            reference: concat!("#/components/schemas/", stringify!($ref)).to_owned()
        })
    };

    (@object_property_value $type:ident => $($tail:tt)*) => {
        ModelReference::Inline(describe_type!($type => $($tail)*))
    }
);

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
                id[required]: (link => TransactionId)
                test[required]: (object => {
                    properties: {
                        sub: (link => TransactionId)
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
}
