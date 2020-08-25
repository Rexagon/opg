#[allow(dead_code)]
mod tests {
    #[opg::path(GET ("some" / {test: i32} / "sdf"): {
        tags: { credits },
        summary: "Get planned payments schedule",
        description: "Test",
        security: { "Bearer" },
        200("OK"): Vec<responses::CreditPlannedScheduleResponseItem>,
    })]
    fn get_some_path() {}

    #[opg::path(POST ("some" / u64): {
        tags: deposits,
        summary: "Create deposit",
        description: "Some description",
        security: "Bearer",
        body: Vec<String>,
        200: String
    })]
    fn post_some_data() {}

    // #[opg::path(GET: {
    //     tags: { credits },
    //     summary: "Get planned payments schedule",
    //     security: { "Bearer" },
    //     200("OK"): Vec<responses::CreditPlannedScheduleResponseItem>,
    // })]
    // fn get_some_path() {}

    #[test]
    fn test_filters() {
        println!("it compiles");
    }
}
