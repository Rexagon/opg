#[allow(dead_code)]
mod tests {
    use opg::*;
    use serde::Serialize;

    #[derive(Serialize, OpgModel)]
    #[opg("New type description", format = "uuid", example = "000-000")]
    struct NewType(String);

    #[derive(Serialize, OpgModel)]
    #[opg("Override description")]
    struct NewNewType(NewType);

    #[test]
    fn test_newtype() {
        assert_eq!(
            serde_yaml::to_string(&NewType::get_structure()).unwrap(),
            r##"---
description: New type description
type: string
format: uuid
example: 000-000"##
        );

        assert_eq!(
            serde_yaml::to_string(&NewNewType::get_structure()).unwrap(),
            r##"---
description: Override description
type: string
format: uuid
example: 000-000"##
        );
    }

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    struct SimpleStruct {
        asd: u32,
        #[opg(optional)]
        hello_camel_case: NewType,
    }

    #[test]
    fn test_simple_struct() {
        assert_eq!(
            serde_yaml::to_string(&SimpleStruct::get_structure()).unwrap(),
            r##"---
type: object
properties:
  asd:
    type: integer
  helloCamelCase:
    $ref: "#/components/schemas/NewType"
required:
  - asd"##
        );
    }

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "kebab-case")]
    #[opg("New type description")]
    enum StringEnumTest {
        First,
        Second,
        HelloWorld,
    }

    #[test]
    fn test_string_enum() {
        assert_eq!(
            serde_yaml::to_string(&StringEnumTest::get_structure()).unwrap(),
            r##"---
description: New type description
type: string
enum:
  - first
  - second
  - hello-world
example: first"##
        );
    }

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "kebab-case")]
    enum ExternallyTaggedEnum {
        Test(String),
        AnotherTest(String, #[opg("Second")] String),
    }

    #[test]
    fn test_externally_enum() {
        assert_eq!(
            serde_yaml::to_string(&ExternallyTaggedEnum::get_structure()).unwrap(),
            r##"---
type: object
additionalProperties:
  oneOf:
    - type: string
    - type: array
      items:
        oneOf:
          - type: string
          - description: Second
            type: string"##
        );
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

    #[test]
    fn test_untagged_enum() {
        assert_eq!(
            serde_yaml::to_string(&UntaggedEnumTest::get_structure()).unwrap(),
            r##"---
oneOf:
  - type: object
    properties:
      value:
        $ref: "#/components/schemas/NewType"
    required:
      - value
  - description: Very simple variant
    type: object
    properties:
      another:
        description: Very simple struct
        type: object
        properties:
          asd:
            type: integer
          helloCamelCase:
            $ref: "#/components/schemas/NewType"
        required:
          - asd
    required:
      - another"##
        );
    }

    #[derive(Serialize, OpgModel)]
    #[serde(tag = "tag", rename_all = "kebab-case")]
    enum InternallyTaggedEnum {
        Test(SimpleStruct),
        AnotherTest { field: String },
    }

    #[test]
    fn test_internally_tagged_enum() {
        assert_eq!(
            serde_yaml::to_string(&InternallyTaggedEnum::get_structure()).unwrap(),
            r##"---
oneOf:
  - type: object
    properties:
      asd:
        type: integer
      helloCamelCase:
        $ref: "#/components/schemas/NewType"
      tag:
        description: InternallyTaggedEnum type variant
        type: string
        enum:
          - test
        example: test
    required:
      - asd
      - tag
  - type: object
    properties:
      field:
        type: string
      tag:
        description: InternallyTaggedEnum type variant
        type: string
        enum:
          - another-test
        example: another-test
    required:
      - field
      - tag"##
        );
    }

    #[derive(Serialize, OpgModel)]
    #[serde(tag = "tag", content = "content", rename_all = "kebab-case")]
    enum AdjacentlyTaggedEnum {
        Test(String),
        AnotherTest(NewType, NewType),
    }

    #[test]
    fn test_adjacently_tagged_enum() {
        assert_eq!(
            serde_yaml::to_string(&AdjacentlyTaggedEnum::get_structure()).unwrap(),
            r##"---
type: object
properties:
  content:
    oneOf:
      - type: string
      - type: array
        items:
          oneOf:
            - $ref: "#/components/schemas/NewType"
            - $ref: "#/components/schemas/NewType"
  tag:
    description: AdjacentlyTaggedEnum type variant
    type: string
    enum:
      - test
      - another-test
    example: test
required:
  - tag
  - content"##
        );
    }

    #[derive(Serialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    struct TypeChangedStruct {
        #[opg(integer)]
        asd: String,
    }

    #[test]
    fn test_type_changed_struct() {
        assert_eq!(
            serde_yaml::to_string(&TypeChangedStruct::get_structure()).unwrap(),
            r##"---
type: object
properties:
  asd:
    type: integer
required:
  - asd"##
        );
    }

    #[test]
    fn test_tuples() {
        assert_eq!(
            serde_yaml::to_string(&<(String, u64)>::get_structure()).unwrap(),
            r##"---
type: array
items:
  oneOf:
    - type: string
    - type: integer"##
        );
    }

    #[derive(Serialize, OpgModel)]
    struct StructWithInner {
        field: Option<String>,
        #[opg(optional)]
        super_optional: Option<Option<String>>,
        boxed: Box<Option<i32>>,
    }

    #[test]
    fn test_inner_type() {
        assert_eq!(
            serde_yaml::to_string(&StructWithInner::get_structure()).unwrap(),
            r##"---
type: object
properties:
  boxed:
    nullable: true
    type: integer
  field:
    nullable: true
    type: string
  super_optional:
    nullable: true
    type: string
required:
  - field
  - boxed"##
        );
    }

    #[test]
    fn test_hash_map() {
        assert_eq!(
            serde_yaml::to_string(&std::collections::HashMap::<&str, i32>::get_structure())
                .unwrap(),
            r##"---
type: object
additionalProperties:
  type: integer"##
        );
    }

    #[derive(Serialize, OpgModel)]
    struct NullableNewtype(Option<i32>);

    #[test]
    fn test_nullable_newtype() {
        assert_eq!(
            serde_yaml::to_string(&NullableNewtype::get_structure()).unwrap(),
            r##"---
nullable: true
type: integer"##
        );
    }

    #[derive(Serialize, OpgModel)]
    struct StructWithNullable {
        #[opg(nullable)]
        field: i32,
    }

    #[test]
    fn test_nullable_field() {
        assert_eq!(
            serde_yaml::to_string(&StructWithNullable::get_structure()).unwrap(),
            r##"---
type: object
properties:
  field:
    nullable: true
    type: integer
required:
  - field"##
        );
    }

    #[test]
    fn test_serialization() {
        let model = Model {
            description: Some("Some type".to_owned()),
            data: ModelData::Single(ModelType {
                nullable: false,
                type_description: ModelTypeDescription::Object(ModelObject {
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
                                data: ModelData::Single(ModelType {
                                    nullable: false,
                                    type_description: ModelTypeDescription::String(ModelString {
                                        variants: None,
                                        data: ModelSimple {
                                            format: None,
                                            example: None,
                                        },
                                    }),
                                }),
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
                }),
            }),
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
}
