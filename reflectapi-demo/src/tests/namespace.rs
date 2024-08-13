async fn empty<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> reflectapi::Empty {
    reflectapi::Empty {}
}

#[test]
fn test_namespace_with_dash() {
    assert_builder_snapshot!(reflectapi::Builder::<()>::new()
        .name("pet-orders".into())
        .route(empty, |b| {
            b.name("jobs-two.pet-orders.list".into())
                .description("desc".into())
        }))
}
