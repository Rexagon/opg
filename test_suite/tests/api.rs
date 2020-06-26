#[allow(dead_code)]
mod tests {
    use opg::*;
    use serde::Serialize;

    #[derive(Serialize, opg::OpgModel)]
    #[serde(rename_all = "camelCase")]
    #[opg("New type description")]
    enum SuperResponse {
        Test,
        Another,
        Yay,
    }

    mod request {
        use super::*;

        #[derive(Serialize, opg::OpgModel)]
        pub struct InModule {
            field: String,
            second: Test,
        }

        #[derive(Serialize, opg::OpgModel)]
        pub struct Test {
            another_field: Option<String>,
        }
    }

    #[test]
    fn expands_normally() {
        let test = describe_api! {
            info: {
                title: "My super API",
                version: "0.0.0",
            },
            tags: {internal, admin("Super admin methods")},
            servers: {
                "https://my.super.server.com/v1",
            },
            paths: {
                ("test"): {
                    POST: {
                        body: {
                            schema: request::InModule,
                        }
                        200("Ok"): std::vec::Vec<String>,
                    }
                },
                ("hello" / "world" / { paramTest: String }): {
                    summary: "Some test group of requests",
                    description: "Another test description",
                    parameters: {
                        (header "x-request-id"): {
                            description: "Test",
                        },
                        (header "test"),
                        (header "asd"),
                    },
                    GET: {
                        tags: {internal},
                        summary: "Small summary",
                        description: "Small description",
                        parameters: {
                            (query someParam: u32): {
                                description: "Test",
                            }
                        }
                        200("Ok"): String,
                    },
                    POST: {
                        tags: {admin},
                        body: {
                            description: "Some interesting description",
                            schema: String,
                            required: true,
                        }
                        200("Ok"): SuperResponse,
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
      requestBody:
        required: true
        description: ""
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/InModule"
      responses:
        200:
          description: Ok
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
          description: Ok
          content:
            application/json:
              schema:
                type: string
      parameters:
        - name: someParam
          description: Test
          in: header
          schema:
            type: integer
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
          description: Ok
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/SuperResponse"
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
        in: header
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
    SuperResponse:
      description: New type description
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
        - another_field"##
        );
    }
}
