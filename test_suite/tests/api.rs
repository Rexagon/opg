use opg::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, OpgModel)]
#[serde(rename_all = "camelCase")]
#[opg("New type description", string)]
#[allow(dead_code)]
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

    println!("{}", serde_yaml::to_string(&test).unwrap());
}
