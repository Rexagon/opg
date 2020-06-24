#[allow(dead_code)]
mod tests {
    use opg::*;
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize, OpgModel)]
    #[serde(rename_all = "camelCase")]
    #[opg("New type description", string)]
    enum SuperResponse {
        Test,
        Another,
        Yay,
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
                ("hello" / "world" / { paramTest: String }): {
                    summary: "Some test group of requests",
                    description: "Another test description",
                    parameters: {
                        (header "x-request-id"): {
                            description: "Test",
                            required: true,
                        }
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
                        200: String ("Ok"),
                    },
                    POST: {
                        tags: {admin},
                        body: {
                            description: "Some interesting description",
                            schema: String,
                            required: true,
                        }
                        200: SuperResponse ("Ok")
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
    SuperResponse:
      description: New type description
      type: string
      enum:
        - test
        - another
        - yay
      example: test"##
        );
    }
}
