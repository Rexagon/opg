#[allow(dead_code)]
mod tests {
    use opg::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    #[opg("Simple enum")]
    enum SimpleEnum {
        Test,
        Another,
        Yay,
    }

    mod request {
        use super::*;

        #[derive(Serialize, OpgModel)]
        pub struct InModule {
            field: String,
            second: Test,
        }

        #[derive(Serialize, OpgModel)]
        pub struct Test {
            another_field: Option<String>,
        }
    }

    #[test]
    fn expands_normally() {
        let test_auth = ApiKeySecurityScheme {
            parameter_in: ParameterIn::Query,
            name: "X-MY-SUPER-API".to_string(),
            description: None,
        };

        let test = describe_api! {
            info: {
                title: "My super API",
                version: "0.0.0",
            },
            tags: {internal, admin("Super admin methods")},
            servers: {
                "https://my.super.server.com/v1"
            },
            security_schemes: {
                (http "bearerAuth"): {
                    scheme: Bearer,
                    bearer_format: "JWT",
                    description: "Test description"
                },
                (http "basicAuth"): {
                    scheme: Basic,
                    description: "Another test description"
                },
                (apiKey "ApiKeyAuth"): {
                    parameter_in: Query,
                    name: "X-API-KEY",
                    description: "And another test description"
                }
            },
            paths: {
                ("test"): {
                    POST: {
                        security: {
                            test_auth && "basicAuth"
                        },
                        body: request::InModule,
                        200: std::vec::Vec<String>
                    }
                },
                ("hello" / "world" / { paramTest: String }): {
                    summary: "Some test group of requests",
                    description: "Another test description",
                    parameters: {
                        (header "x-request-id"): {
                            description: "Test",
                        },
                        (query test: i32),
                        (header "asd")
                    },
                    GET: {
                        tags: {internal},
                        summary: "Small summary",
                        description: "Small description",
                        parameters: {
                            (query someParam: u32): {
                                description: "Test",
                            },
                        },
                        200("Custom response desc"): String
                    },
                    POST: {
                        tags: {admin},
                        body: {
                            description: "Some interesting description",
                            schema: String,
                            required: true
                        },
                        200: SimpleEnum,
                    },
                    DELETE: {
                        200: None,
                    },
                    OPTIONS: {
                        200: ()
                    }
                }
            }
        };

        assert_eq!(
            serde_yaml::to_string(&test).unwrap(),
            r##"---
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
  /test:
    post:
      security:
        - basicAuth: []
          test_auth: []
      requestBody:
        required: true
        description: ""
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/InModule"
      responses:
        200:
          description: OK
          content:
            application/json:
              schema:
                type: array
                items:
                  type: string
  "/hello/world/{paramTest}":
    summary: Some test group of requests
    description: Another test description
    get:
      tags:
        - internal
      summary: Small summary
      description: Small description
      responses:
        200:
          description: Custom response desc
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
      requestBody:
        required: true
        description: Some interesting description
        content:
          application/json:
            schema:
              type: string
      responses:
        200:
          description: OK
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/SimpleEnum"
    delete:
      responses:
        200:
          description: OK
    options:
      responses:
        200:
          description: OK
          content:
            application/json:
              schema:
                description: "Always `null`"
                nullable: true
                type: string
                format: "null"
    parameters:
      - name: asd
        in: header
        required: true
        schema:
          type: string
      - name: paramTest
        in: path
        required: true
        schema:
          type: string
      - name: test
        in: query
        schema:
          type: integer
          format: int32
      - name: x-request-id
        description: Test
        in: header
        required: true
        schema:
          type: string
components:
  schemas:
    InModule:
      type: object
      properties:
        field:
          type: string
        second:
          $ref: "#/components/schemas/Test"
      required:
        - field
        - second
    SimpleEnum:
      description: Simple enum
      type: string
      enum:
        - test
        - another
        - yay
      example: test
    Test:
      type: object
      properties:
        another_field:
          nullable: true
          type: string
      required:
        - another_field
  securitySchemes:
    ApiKeyAuth:
      type: apiKey
      in: query
      name: X-API-KEY
      description: And another test description
    basicAuth:
      type: http
      scheme: basic
      description: Another test description
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
      description: Test description
    test_auth:
      type: apiKey
      in: query
      name: X-MY-SUPER-API"##
        );
    }
}
