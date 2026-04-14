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
fn test_conflicting_namespace_names() {
    // Make sure the two handlers have incompatible types for this test to be valid.
    assert_builder_snapshot!(reflectapi::Builder::<()>::new()
        .name("foo")
        .route(
            |_, _: reflectapi::Empty, _: reflectapi::Empty| async { reflectapi::Empty {} },
            |b| b.name("x.foo.get")
        )
        .route(
            |_, _: reflectapi::Empty, _: reflectapi::Empty| async {},
            |b| b.name("y.foo.get")
        ))
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

#[test]
fn test_python_destutter_collision_falls_back_to_original_name() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct First {
        first: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Second {
        second: u8,
    }

    async fn first_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> First {
        First {
            first: String::new(),
        }
    }

    async fn second_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> Second {
        Second { second: 0 }
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(first_get, |b| b.name("first.get"))
        .route(second_get, |b| b.name("second.get"))
        .rename_types(
            "reflectapi_demo::tests::namespace::First",
            "OfferRequestPartIdentity",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::Second",
            "offer_request::OfferRequestPartIdentity",
        )
        .build()
        .unwrap();

    let python = reflectapi::codegen::python::generate(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();

    assert_eq!(
        python
            .matches("class OfferRequestPartIdentity(BaseModel):")
            .count(),
        1
    );
    assert!(python.contains("class OfferRequestOfferRequestPartIdentity(BaseModel):"));
    assert!(python.contains("OfferRequestPartIdentity = OfferRequestOfferRequestPartIdentity"));
}

#[test]
fn test_python_init_exports_client() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct TestType {
        value: u8,
    }

    let files = reflectapi::codegen::python::generate_files(
        super::into_schema::<TestType>(),
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();
    let init_py = files.get("__init__.py").unwrap();

    assert!(init_py.contains("from .generated import AsyncClient, Client"));
    assert!(init_py.contains("__all__ = [\"AsyncClient\", \"Client\"]"));
    assert!(!init_py.contains("SyncClient"));
}
