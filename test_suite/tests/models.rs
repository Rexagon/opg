use opg::*;
use serde::Serialize;

#[derive(Serialize, OpgModel)]
#[opg("New type description", format = "uuid", example = "000-000")]
struct NewType(String);

#[derive(Serialize, OpgModel)]
#[opg("Override description")]
struct NewNewType(NewType);

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
struct SimpleStruct {
    asd: u32,
    #[opg(optional)]
    hello_camel_case: NewType,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "kebab-case")]
#[opg("New type description", string)]
enum StringEnumTest {
    First,
    Second,
    HelloWorld,
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "kebab-case")]
enum ExternallyTaggedEnum {
    Test(String),
    AnotherTest(#[opg(inline)] String, String),
}

#[derive(Serialize, OpgModel)]
#[serde(untagged)]
enum UntaggedEnumTest {
    First {
        value: NewType,
    },
    #[opg("Very simple variant")]
    Second {
        #[opg("Very simple struct", inline)]
        another: SimpleStruct,
    },
}

#[derive(Serialize, OpgModel)]
#[serde(tag = "tag", rename_all = "kebab-case")]
enum InternallyTaggedEnum {
    Test(SimpleStruct),
    AnotherTest { field: String },
}

#[derive(Serialize, OpgModel)]
#[serde(tag = "tag", content = "content", rename_all = "kebab-case")]
enum AdjacentlyTaggedEnum {
    Test(String),
    AnotherTest(NewType, NewType),
}

#[derive(Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
struct TypeChangedStruct {
    #[opg(string)]
    asd: String,
}

#[test]
fn test_newtype() {
    println!(
        "{}",
        serde_yaml::to_string(&NewType::get_structure()).unwrap()
    );
    println!(
        "{}",
        serde_yaml::to_string(&NewNewType::get_structure()).unwrap()
    );
}

#[test]
fn test_simple_struct() {
    println!(
        "{}",
        serde_yaml::to_string(&SimpleStruct::get_structure()).unwrap()
    );
}

#[test]
fn test_string_enum() {
    println!(
        "{}",
        serde_yaml::to_string(&StringEnumTest::get_structure()).unwrap()
    );
}

#[test]
fn test_untagged_enum() {
    println!(
        "{}",
        serde_yaml::to_string(&UntaggedEnumTest::get_structure()).unwrap()
    );
}

#[test]
fn test_externally_enum() {
    println!(
        "{}",
        serde_yaml::to_string(&ExternallyTaggedEnum::get_structure()).unwrap()
    );
}

#[test]
fn test_internally_tagged_enum() {
    println!(
        "{}",
        serde_yaml::to_string(&InternallyTaggedEnum::get_structure()).unwrap()
    );
}

#[test]
fn test_adjacently_tagged_enum() {
    println!(
        "{}",
        serde_yaml::to_string(&AdjacentlyTaggedEnum::get_structure()).unwrap()
    );
}

#[test]
fn test_type_changed_struct() {
    println!(
        "{}",
        serde_yaml::to_string(&TypeChangedStruct::get_structure()).unwrap()
    );
}

#[test]
fn test_serialization() {
    let model = Model {
        description: Some("Some type".to_owned()),
        data: ModelData::Single(ModelTypeDescription::Object(ModelObject {
            properties: {
                let mut properties = std::collections::BTreeMap::new();
                properties.insert(
                    "id".to_owned(),
                    ModelReference::Link("TransactionId".to_owned()),
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
    let mut cx = OpgComponents::new();

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

    assert_eq!(cx.verify_schemas(), Ok(()));
}

#[test]
fn test_invalid_models_context() {
    let mut cx = OpgComponents::new();

    let invalid_link = "TransactionId";

    cx.add_model(
        "SomeResponse",
        describe_type!(object => {
            properties: {
                id: (link => invalid_link)
            }
        }),
    );

    assert_eq!(cx.verify_schemas(), Err(invalid_link.to_owned()));
}
