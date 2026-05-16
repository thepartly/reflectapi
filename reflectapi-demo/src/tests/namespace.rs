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

    assert!(init_py.contains("from ._client import AsyncClient, Client"));
    assert!(init_py.contains("__all__ = [\"AsyncClient\", \"Client\", \"reflectapi_demo\"]"));
    assert!(!init_py.contains("SyncClient"));

    let namespace_file = files
        .get("reflectapi_demo/tests/namespace/__init__.py")
        .unwrap();
    assert!(namespace_file.contains("class ReflectapiDemoTestsNamespaceTestType(BaseModel):"));
    assert!(namespace_file.contains("TestType = ReflectapiDemoTestsNamespaceTestType"));
    assert!(namespace_file.contains("__all__"));
    let client_file = files.get("_client.py").unwrap();
    assert!(client_file.contains("from . import reflectapi_demo"));
    assert!(client_file.contains("from ._rebuild import rebuild_models as _rebuild_models"));
    assert!(files.values().all(|src| !src.contains("Namespace classes")));
    assert!(files
        .values()
        .all(|src| !src.contains("class reflectapi_demo:")));
}

#[test]
fn test_python_split_modules_bind_referenced_namespaces() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    enum Status {
        Pending,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Item {
        value: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct OutputChild {
        value: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct OutputSummary {
        child: OutputChild,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Order {
        status: Status,
        items: Vec<Item>,
        summary: OutputSummary,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct RootWrapper {
        order: Order,
    }

    async fn order_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> Order {
        Order {
            status: Status::Pending,
            items: vec![],
            summary: OutputSummary {
                child: OutputChild {
                    value: String::new(),
                },
            },
        }
    }

    async fn wrapper_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> RootWrapper {
        RootWrapper {
            order: Order {
                status: Status::Pending,
                items: vec![],
                summary: OutputSummary {
                    child: OutputChild {
                        value: String::new(),
                    },
                },
            },
        }
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(order_get, |b| b.name("order.get"))
        .route(wrapper_get, |b| b.name("wrapper.get"))
        .rename_types(
            "reflectapi_demo::tests::namespace::Status",
            "lookup::Status",
        )
        .rename_types("reflectapi_demo::tests::namespace::Item", "order::Item")
        .rename_types("reflectapi_demo::tests::namespace::Order", "order::Order")
        .rename_types(
            "reflectapi_demo::tests::namespace::RootWrapper",
            "RootWrapper",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::OutputChild",
            "order::output::Child",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::OutputSummary",
            "order::output::Summary",
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();
    let order_file = files.get("order/__init__.py").unwrap();

    assert!(order_file.contains("import sys"));
    assert!(order_file.contains("from .. import lookup"));
    assert!(order_file.contains("order = sys.modules[__name__]"));
    assert!(order_file.contains("from . import output"));
    assert!(order_file.contains("status: lookup.Status"));
    assert!(order_file.contains("items: list[Item]"));
    assert!(order_file.contains("summary: output.Summary"));
    assert!(order_file.contains("defer_build=True"));
    assert!(!order_file.contains("_rebuild_models()"));

    let output_file = files.get("order/output/__init__.py").unwrap();
    assert!(output_file.contains("import sys"));
    assert!(output_file.contains("from ... import order"));
    assert!(output_file.contains("order.output = sys.modules[__name__]"));
    assert!(output_file.contains("child: Child"));
    assert!(output_file.contains("defer_build=True"));

    let root_types_file = files.get("_types.py").unwrap();
    assert!(root_types_file.contains("from typing import TYPE_CHECKING"));
    assert!(root_types_file.contains("if TYPE_CHECKING:"));
    assert!(root_types_file.contains("from . import order"));
    assert!(root_types_file.contains("order: order.Order"));
}

#[test]
fn test_python_split_modules_defer_peer_namespace_imports() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    enum Metric {
        Price,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Sort<T> {
        value: T,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Criterion {
        sort: Sort<Metric>,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Basket {
        criterion: Criterion,
    }

    async fn basket_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> Basket {
        Basket {
            criterion: Criterion {
                sort: Sort {
                    value: Metric::Price,
                },
            },
        }
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(basket_get, |b| b.name("basket.get"))
        .rename_types(
            "reflectapi_demo::tests::namespace::Basket",
            "basket_builder::Basket",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::Sort",
            "basket_builder::Sort",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::Criterion",
            "basket_builder_basket::Criterion",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::Metric",
            "basket_builder_basket::Metric",
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();
    let basket_builder_file = files.get("basket_builder/__init__.py").unwrap();
    let basket_builder_import = basket_builder_file
        .find("from .. import basket_builder_basket")
        .unwrap();
    let sort_alias = basket_builder_file
        .find("Sort = BasketBuilderSort")
        .unwrap();
    assert!(basket_builder_import > sort_alias);
    assert!(basket_builder_file.contains("criterion: basket_builder_basket.Criterion"));

    let basket_file = files.get("basket_builder_basket/__init__.py").unwrap();
    let parent_import = basket_file.find("from .. import basket_builder").unwrap();
    let metric_alias = basket_file
        .find("Metric = BasketBuilderBasketMetric")
        .unwrap();
    assert!(parent_import > metric_alias);
    assert!(basket_file.contains("sort: basket_builder.Sort[Metric]"));
}

#[test]
fn test_python_split_modules_placeholder_deferred_builtin_namespaces() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct PricingModel {
        value: u8,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct BillingRequest {
        pricing_model: PricingModel,
    }

    async fn billing_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> BillingRequest {
        BillingRequest {
            pricing_model: PricingModel { value: 1 },
        }
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(billing_get, |b| b.name("billing.get"))
        .rename_types(
            "reflectapi_demo::tests::namespace::BillingRequest",
            "billing::Request",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::PricingModel",
            "billing::input::PricingModel",
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();
    let billing_file = files.get("billing/__init__.py").unwrap();
    let placeholder = billing_file
        .find("input = _ReflectapiDeferredNamespace()")
        .unwrap();
    let request_class = billing_file.find("class BillingRequest").unwrap();
    let child_import = billing_file.find("from . import input").unwrap();

    assert!(placeholder < request_class);
    assert!(child_import > request_class);
    assert!(billing_file.contains("pricing_model: input.PricingModel"));
}

#[test]
fn test_python_module_path_collision_errors() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct First {
        value: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Second {
        value: String,
    }

    async fn first_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> First {
        First {
            value: String::new(),
        }
    }

    async fn second_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> Second {
        Second {
            value: String::new(),
        }
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(first_get, |b| b.name("first.get"))
        .route(second_get, |b| b.name("second.get"))
        .rename_types("reflectapi_demo::tests::namespace::First", "foo-bar::First")
        .rename_types(
            "reflectapi_demo::tests::namespace::Second",
            "foo_bar::Second",
        )
        .build()
        .unwrap();

    let error = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("Python module path collision"));
}
