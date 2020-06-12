use std::collections::BTreeMap;

use serde::Serialize;

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
    pub items: Vec<ModelReference>,
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
    (string => {
        $(description: $description:literal)?
        $(format: $format:literal)?
        $(example: $example:literal)?
        $(variants: [$($variants:literal),*])?
    }) => {
        Model {
            description: ($($description.to_string(),)?).extract(),
            data: ModelData::Single(ModelTypeDescription::String(ModelString {
                variants: ($(vec![$($variants.to_string()),*])?,).extract(),
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
        let test = describe_type!(object => {
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
            }
        });

        println!("{}", serde_yaml::to_string(&test).unwrap());

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
}
