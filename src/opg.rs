use std::collections::BTreeMap;

use serde::Serialize;

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
    pub example: Option<Vec<String>>,
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
}
