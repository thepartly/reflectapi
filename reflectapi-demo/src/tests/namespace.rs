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

// Regression for rust client codegen where field `T` would resolve to the type `T` instead of the
// type parameter `T`.
#[test]
fn test_generic_and_type_conflict() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct T<I>(I);

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct K<T> {
        t: T,
    }

    async fn get<S>(_s: S, k: K<T<()>>, _: reflectapi::Empty) -> K<T<()>> {
        k
    }

    assert_builder_snapshot!(reflectapi::Builder::<()>::new()
        .route(get, |b| b.name("get"))
        .rename_types("reflectapi_demo::tests::namespace::T", "T"))
}
