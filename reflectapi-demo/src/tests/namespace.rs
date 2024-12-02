#[test]
fn test_namespace_with_dash() {
    async fn empty<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> reflectapi::Empty {
        reflectapi::Empty {}
    }

    assert_builder_snapshot!(reflectapi::Builder::<()>::new()
        .name("pet-orders")
        .route(empty, |b| {
            b.name("jobs-two.pet-orders.list-x").description("desc")
        }))
}

#[test]
fn test_conflicting_names() {
    #[derive(Debug, serde::Serialize, reflectapi::Output)]
    struct Foo {}

    async fn foos_get<S>(_s: S, _: usize, _: reflectapi::Empty) -> Foo {
        Foo {}
    }

    assert_builder_snapshot!(reflectapi::Builder::<()>::new()
        .name("foos")
        .route(foos_get, |b| b.name("foos.get"))
        .rename_types("reflectapi_demo::tests::namespace::Foo", "foos::Foo"))
}
