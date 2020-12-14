<p align="center">
    <h3 align="center">opg</h3>
    <p align="center">Rust OpenAPI 3.0 docs generator</p>
    <p align="center">
        <a href="/LICENSE">
            <img alt="GitHub" src="https://img.shields.io/github/license/Rexagon/opg" />
        </a>
        <a href="https://github.com/Rexagon/opg/actions?query=workflow%3Amaster">
            <img alt="GitHub Workflow Status" src="https://img.shields.io/github/workflow/status/Rexagon/opg/master" />
        </a>
        <a href="https://crates.io/crates/opg">
            <img alt="Crates.io Version" src="https://img.shields.io/crates/v/opg" />
        </a>
        <a href="https://docs.rs/opg">
            <img alt="Docs.rs" src="https://docs.rs/opg/badge.svg" />
        </a>
    </p>
</p>

#### Example:
> Or see more [here](https://github.com/Rexagon/opg/tree/master/test_suite/tests)

```rust
use opg::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("Simple enum")]
enum SimpleEnum {
    Test,
    Another,
    Yay,
}

#[derive(Serialize, Deserialize, OpgModel)]
#[opg("newtype string", format = "id", example = "abcd0001")]
struct NewType(String);

#[derive(Serialize, Deserialize, OpgModel)]
struct SimpleStruct {
    first_field: i32,
    #[opg("Field description")]
    second: String,
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "kebab-case")]
enum ExternallyTaggedEnum {
    Test(String),
    AnotherTest(String, #[opg("Second")] String),
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(untagged)]
enum UntaggedEnum {
    First {
        value: NewType,
    },
    #[opg("Variant description")]
    Second {
        #[opg("Inlined struct", inline)]
        another: SimpleStruct,
    },
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(tag = "tag", rename_all = "lowercase")]
enum InternallyTaggedEnum {
    First(SimpleStruct),
    Second { field: String },
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(tag = "tag", content = "content", rename_all = "lowercase")]
enum AdjacentlyTaggedEnum {
    First(String),
    Second(NewType, NewType),
}

#[derive(Serialize, Deserialize, OpgModel)]
#[serde(rename_all = "camelCase")]
struct TypeChangedStruct {
    #[serde(with = "chrono::naive::serde::ts_milliseconds")]
    #[opg("UTC timestamp in milliseconds", integer, format = "int64")]
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, OpgModel)]
struct StructWithComplexObjects {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[opg(optional)]
    super_optional: Option<Option<String>>,
    field: Option<String>,
    boxed: Box<Option<i32>>,
}

#[derive(Serialize, OpgModel)]
struct GenericStructWithRef<'a, T> {
    message: &'a str,
    test: T,
}

#[derive(Serialize, OpgModel)]
struct SuperResponse {
    simple_enum: SimpleEnum,
    #[serde(rename = "new_type")]
    newtype: NewType,
    externally_tagged_enum: ExternallyTaggedEnum,
    untagged_enum: UntaggedEnum,
    internally_tagged_enum: InternallyTaggedEnum,
    adjacently_tagged_enum: AdjacentlyTaggedEnum,
    type_changed_struct: TypeChangedStruct,
    struct_with_complex_objects: StructWithComplexObjects,
}

#[test]
fn print_api() {
    let test = describe_api! {
        info: {
            title: "My super API",
            version: "0.0.0",
        },
        tags: {internal, admin("Super admin methods")},
        servers: {
            "https://my.super.server.com/v1",
        },
        security_schemes: {
            (http "bearerAuth"): {
                scheme: Bearer,
                bearer_format: "JWT",
            },
        },
        paths: {
            ("hello" / "world" / { paramTest: String }): {
                summary: "Some test group of requests",
                description: "Another test description",
                parameters: {
                    (header "x-request-id"): {
                        description: "Test",
                        required: true,
                    },
                },
                GET: {
                    tags: {internal},
                    summary: "Small summary",
                    description: "Small description",
                    deprecated: true,
                    parameters: {
                        (query someParam: u32): {
                            description: "Test",
                        }
                    },
                    200: String,
                },
                POST: {
                    tags: {admin},
                    security: {"basicAuth"},
                    body: {
                        description: "Some interesting description",
                        schema: GenericStructWithRef<'static, i64>,
                        required: true,
                    },
                    200: SuperResponse,
                }
            }
        }
    };

    println!("{}", serde_yaml::to_string(&test).unwrap());
}
```

<details><summary><b>Result:</b></summary>
<p>

```yaml
---
openapi: 3.0.3
info:
  title: My super API
  version: 0.0.0
tags:
  - name: admin
    description: Super admin methods
  - name: internal
servers:
  - url: "https://my.super.server.com/v1"
paths:
  "/hello/world/{paramTest}":
    summary: Some test group of requests
    description: Another test description
    get:
      tags:
        - internal
      summary: Small summary
      description: Small description
      deprecated: true
      responses:
        200:
          description: OK
          content:
            application/json:
              schema:
                type: string
      parameters:
        - name: someParam
          description: Test
          in: query
          schema:
            type: integer
            format: uint32
    post:
      tags:
        - admin
      security:
        - basicAuth: []
      requestBody:
        required: true
        description: Some interesting description
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/GenericStructWithRef"
      responses:
        200:
          description: OK
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/SuperResponse"
    parameters:
      - name: paramTest
        in: path
        required: true
        schema:
          type: string
      - name: x-request-id
        description: Test
        in: header
        required: true
        schema:
          type: string
components:
  schemas:
    AdjacentlyTaggedEnum:
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
            - first
            - second
          example: first
      required:
        - tag
        - content
    ExternallyTaggedEnum:
      type: object
      additionalProperties:
        oneOf:
          - type: string
          - type: array
            items:
              oneOf:
                - type: string
                - description: Second
                  type: string
    GenericStructWithRef:
      type: object
      properties:
        message:
          type: string
        test:
          type: integer
          format: int64
      required:
        - message
        - test
    InternallyTaggedEnum:
      oneOf:
        - type: object
          properties:
            first_field:
              type: integer
              format: int32
            second:
              description: Field description
              type: string
            tag:
              description: InternallyTaggedEnum type variant
              type: string
              enum:
                - first
              example: first
          required:
            - first_field
            - second
            - tag
        - type: object
          properties:
            field:
              type: string
            tag:
              description: InternallyTaggedEnum type variant
              type: string
              enum:
                - second
              example: second
          required:
            - field
            - tag
    NewType:
      description: newtype string
      type: string
      format: id
      example: abcd0001
    SimpleEnum:
      description: Simple enum
      type: string
      enum:
        - test
        - another
        - yay
      example: test
    StructWithComplexObjects:
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
        - boxed
    SuperResponse:
      type: object
      properties:
        adjacently_tagged_enum:
          $ref: "#/components/schemas/AdjacentlyTaggedEnum"
        externally_tagged_enum:
          $ref: "#/components/schemas/ExternallyTaggedEnum"
        internally_tagged_enum:
          $ref: "#/components/schemas/InternallyTaggedEnum"
        new_type:
          $ref: "#/components/schemas/NewType"
        simple_enum:
          $ref: "#/components/schemas/SimpleEnum"
        struct_with_complex_objects:
          $ref: "#/components/schemas/StructWithComplexObjects"
        type_changed_struct:
          $ref: "#/components/schemas/TypeChangedStruct"
        untagged_enum:
          $ref: "#/components/schemas/UntaggedEnum"
      required:
        - simple_enum
        - new_type
        - externally_tagged_enum
        - untagged_enum
        - internally_tagged_enum
        - adjacently_tagged_enum
        - type_changed_struct
        - struct_with_complex_objects
    TypeChangedStruct:
      type: object
      properties:
        timestamp:
          description: UTC timestamp in milliseconds
          type: integer
          format: int64
      required:
        - timestamp
    UntaggedEnum:
      oneOf:
        - type: object
          properties:
            value:
              $ref: "#/components/schemas/NewType"
          required:
            - value
        - description: Variant description
          type: object
          properties:
            another:
              description: Inlined struct
              type: object
              properties:
                first_field:
                  type: integer
                  format: int32
                second:
                  description: Field description
                  type: string
              required:
                - first_field
                - second
          required:
            - another
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
```
</p>
</details>
