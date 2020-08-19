#[allow(dead_code)]
mod tests {
    use opg::{Components, OpgModel};
    use serde::Serialize;

    #[derive(Serialize, OpgModel)]
    #[opg("New type description", format = "uuid", example = "000-000")]
    struct NewType(String);

    #[derive(Serialize, OpgModel)]
    #[opg("Override description")]
    struct NewNewType(NewType);

    #[test]
    fn newtype() {
        let mut cx = Components::new();

        assert_eq!(
            serde_yaml::to_string(&NewType::get_schema(&mut cx)).unwrap(),
            r##"---
description: New type description
type: string
format: uuid
example: 000-000"##
        );

        assert_eq!(
            serde_yaml::to_string(&NewNewType::get_schema(&mut cx)).unwrap(),
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
        asd: i32,
        #[opg(optional)]
        hello_camel_case: NewType,
    }

    #[test]
    fn simple_struct() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&SimpleStruct::get_schema(&mut cx)).unwrap(),
            r##"---
type: object
properties:
  asd:
    type: integer
    format: int32
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
    fn string_enum() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&StringEnumTest::get_schema(&mut cx)).unwrap(),
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
    fn externally_enum() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&ExternallyTaggedEnum::get_schema(&mut cx)).unwrap(),
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
    fn untagged_enum() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&UntaggedEnumTest::get_schema(&mut cx)).unwrap(),
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
            format: int32
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
    fn internally_tagged_enum() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&InternallyTaggedEnum::get_schema(&mut cx)).unwrap(),
            r##"---
oneOf:
  - type: object
    properties:
      asd:
        type: integer
        format: int32
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
    fn adjacently_tagged_enum() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&AdjacentlyTaggedEnum::get_schema(&mut cx)).unwrap(),
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
    fn type_changed_field() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&TypeChangedStruct::get_schema(&mut cx)).unwrap(),
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
    fn tuples() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&<(String, u64)>::get_schema(&mut cx)).unwrap(),
            r##"---
type: array
items:
  oneOf:
    - type: string
    - type: integer
      format: uint64"##
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
    fn inner_type() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&StructWithInner::get_schema(&mut cx)).unwrap(),
            r##"---
type: object
properties:
  boxed:
    nullable: true
    type: integer
    format: int32
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
    fn hash_map() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&std::collections::HashMap::<&str, i32>::get_schema(&mut cx))
                .unwrap(),
            r##"---
type: object
additionalProperties:
  type: integer
  format: int32"##
        );
    }

    #[derive(Serialize, OpgModel)]
    struct NullableNewtype(Option<i32>);

    #[test]
    fn nullable_newtype() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&NullableNewtype::get_schema(&mut cx)).unwrap(),
            r##"---
nullable: true
type: integer
format: int32"##
        );
    }

    #[derive(Serialize, OpgModel)]
    struct StructWithNullable {
        #[opg(nullable)]
        field: i32,
    }

    #[test]
    fn nullable_field() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&StructWithNullable::get_schema(&mut cx)).unwrap(),
            r##"---
type: object
properties:
  field:
    nullable: true
    type: integer
    format: int32
required:
  - field"##
        );
    }

    #[derive(Serialize, OpgModel)]
    struct Recursive {
        recursive_field: Option<Box<Recursive>>,
    }

    fn recursive_field() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&Recursive::get_schema(&mut cx)).unwrap(),
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

    #[derive(Debug, Serialize, Hash, OpgModel)]
    #[serde(rename_all = "snake_case")]
    #[opg("Credit history kind")]
    pub enum CreditHistoryMetaResponse {
        #[serde(rename_all = "camelCase")]
        CreateCredit {
            pledge_currency: Option<String>,
            pledge_amount: Option<String>,
            credit_currency: String,
            credit_amount: String,
        },
        CloseCredit,
        #[serde(rename_all = "camelCase")]
        FreezePledge {
            pledge_currency: String,
            pledge_amount: String,
        },
    }

    #[derive(Debug, Serialize, OpgModel)]
    #[opg("Test")]
    struct GenericStruct<T> {
        body: T,
    }

    #[test]
    fn generic_struct() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&GenericStruct::<i32>::get_schema(&mut cx)).unwrap(),
            r##"---
description: Test
type: object
properties:
  body:
    type: integer
    format: int32
required:
  - body"##
        );
    }

    #[derive(Debug, Serialize, OpgModel)]
    struct GenericStructWithRef<'a, T> {
        message: &'a str,
        test: T,
    }

    #[test]
    fn generic_struct_with_ref() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&GenericStructWithRef::<i32>::get_schema(&mut cx)).unwrap(),
            r##"---
type: object
properties:
  message:
    type: string
  test:
    type: integer
    format: int32
required:
  - message
  - test"##
        );
    }

    #[derive(Debug, Serialize, OpgModel)]
    struct StructWithAny {
        #[opg(any)]
        field: serde_yaml::Value,
    }

    #[test]
    fn struct_with_any_field() {
        let mut cx = Components::default();
        assert_eq!(
            serde_yaml::to_string(&StructWithAny::get_schema(&mut cx)).unwrap(),
            r##"---
type: object
properties:
  field: {}
required:
  - field"##
        );
    }

    #[test]
    fn null_response() {
        let mut cx = Components::default();
        assert_eq!(
            serde_yaml::to_string(&<()>::get_schema(&mut cx)).unwrap(),
            r##"---
description: "Always `null`"
nullable: true
type: string
format: "null""##
        );
    }

    #[test]
    fn complex_enum() {
        let mut cx = &mut Components::default();
        assert_eq!(
            serde_yaml::to_string(&CreditHistoryMetaResponse::get_schema(&mut cx)).unwrap(),
            r##"---
description: Credit history kind
type: object
additionalProperties:
  description: Credit history kind
  oneOf:
    - type: object
      properties:
        creditAmount:
          type: string
        creditCurrency:
          type: string
        pledgeAmount:
          nullable: true
          type: string
        pledgeCurrency:
          nullable: true
          type: string
      required:
        - pledgeCurrency
        - pledgeAmount
        - creditCurrency
        - creditAmount
    - type: string
      enum:
        - close_credit
      example: close_credit
    - type: object
      properties:
        pledgeAmount:
          type: string
        pledgeCurrency:
          type: string
      required:
        - pledgeCurrency
        - pledgeAmount"##
        );
    }

    #[test]
    fn manual_serialization() {
        use opg::*;

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
    fn describe_type_macro() {
        use opg::describe_type;

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
    fn valid_models_context() {
        use opg::describe_type;

        let mut cx = Components::new();

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
    fn invalid_models_context() {
        use opg::describe_type;

        let mut cx = Components::new();

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
