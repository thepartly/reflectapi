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

/// Regression for #161: in the multi-file (package) Python client a root
/// type and a same-leaf type under a sub-namespace must not collide on a
/// bare public name. Previously each namespace emitted a `<Leaf> =
/// <NamespacePrefixedLeaf>` rebind that shadowed the `from .._types import
/// <Leaf>` it also emitted, so `_types.<Leaf>` and `<ns>.<Leaf>` resolved to
/// two distinct classes and field annotations silently bound to the wrong
/// one (pydantic then rejected values with a self-identical error message).
#[test]
fn test_python_namespace_leaf_collision_no_shadowing() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct SomeMatch {
        id: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct RootConflict<C> {
        not_exists: Option<C>,
        matches: Option<SomeMatch>,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct NsConflict<C> {
        not_exists: Option<C>,
        matches: Option<SomeMatch>,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct TopWrapper {
        conflict: RootConflict<SomeMatch>,
        ns_conflict: NsConflict<SomeMatch>,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct NomatchesUpdateRequest {
        conflict: NsConflict<SomeMatch>,
    }

    async fn top_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> TopWrapper {
        unimplemented!()
    }
    async fn nomatches_update<S>(
        _s: S,
        _: NomatchesUpdateRequest,
        _: reflectapi::Empty,
    ) -> reflectapi::Empty {
        reflectapi::Empty {}
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(top_get, |b| b.name("top.get"))
        .route(nomatches_update, |b| b.name("nomatches.update"))
        .rename_types(
            "reflectapi_demo::tests::namespace::TopWrapper",
            "TopWrapper",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::NomatchesUpdateRequest",
            "nomatches::UpdateRequest",
        )
        .rename_types("reflectapi_demo::tests::namespace::SomeMatch", "SomeMatch")
        .rename_types(
            "reflectapi_demo::tests::namespace::RootConflict",
            "IfConflictOnUpdate",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::NsConflict",
            "nomatches::IfConflictOnUpdate",
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();

    let types_py = files.get("_types.py").unwrap();
    let nomatches_py = files.get("nomatches/__init__.py").unwrap();

    // The root type is defined once (in _types.py); the namespace type keeps
    // its globally-unique, namespace-prefixed flat name.
    assert!(types_py.contains("class IfConflictOnUpdate(BaseModel"));
    assert!(nomatches_py.contains("class NomatchesIfConflictOnUpdate(BaseModel"));
    assert!(nomatches_py.contains("from .._types import"));

    // The namespace must NOT rebind the bare `IfConflictOnUpdate` it imports
    // from `_types` to a *different* local class — that shadowing is the #161
    // bug (`_types.X is not nomatches.X`).
    assert!(
        !nomatches_py.contains("IfConflictOnUpdate = NomatchesIfConflictOnUpdate"),
        "namespace must not shadow the imported root `IfConflictOnUpdate`:\n{nomatches_py}"
    );

    // The namespace wrapper's field must bind to the disambiguated local
    // class, not the bare (formerly shadowed) leaf.
    assert!(
        nomatches_py.contains("conflict: NomatchesIfConflictOnUpdate["),
        "namespace field must reference the disambiguated local class:\n{nomatches_py}"
    );

    // A root type that references the namespace-local type must qualify it
    // with the disambiguated name; `nomatches.IfConflictOnUpdate` now resolves
    // to the imported root class, so it would otherwise be the wrong type.
    assert!(
        types_py.contains("nomatches.NomatchesIfConflictOnUpdate["),
        "root cross-namespace ref must use the disambiguated name:\n{types_py}"
    );
    assert!(!types_py.contains("nomatches.IfConflictOnUpdate["));
}

/// Regression for #161 (Codex review follow-up): the collision can be hidden in
/// the *Rust-leaf* alias rather than the prefix-stripped one. A namespaced
/// `myapi::order::OrderInsertData` de-stutters to flat `MyapiOrderInsertData`,
/// so the stripped alias is the harmless `InsertData` while the Rust-leaf alias
/// `OrderInsertData = MyapiOrderInsertData` is what shadows the imported root
/// `OrderInsertData`. Collision detection must cover both alias forms.
#[test]
fn test_python_namespace_destutter_leaf_collision_no_shadowing() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct RootOrderInsertData {
        id: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct NsOrderInsertData {
        name: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct TopWrapper {
        root: RootOrderInsertData,
        ns: NsOrderInsertData,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct OrderUpdateRequest {
        data: NsOrderInsertData,
    }

    async fn top_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> TopWrapper {
        unimplemented!()
    }
    async fn order_update<S>(
        _s: S,
        _: OrderUpdateRequest,
        _: reflectapi::Empty,
    ) -> reflectapi::Empty {
        reflectapi::Empty {}
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(top_get, |b| b.name("top.get"))
        .route(order_update, |b| b.name("myapi.order.update"))
        .rename_types(
            "reflectapi_demo::tests::namespace::TopWrapper",
            "TopWrapper",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::OrderUpdateRequest",
            "myapi::order::UpdateRequest",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::RootOrderInsertData",
            "OrderInsertData",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::NsOrderInsertData",
            "myapi::order::OrderInsertData",
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();

    let types_py = files.get("_types.py").unwrap();
    let order_py = files.get("myapi/order/__init__.py").unwrap();

    assert!(order_py.contains("class MyapiOrderInsertData(BaseModel"));
    // The Rust-leaf alias must not rebind (shadow) the imported root type.
    assert!(
        !order_py.contains("OrderInsertData = MyapiOrderInsertData"),
        "Rust-leaf alias must not shadow the imported root `OrderInsertData`:\n{order_py}"
    );
    // The same-module field reference resolves to the disambiguated local class.
    assert!(
        order_py.contains("data: MyapiOrderInsertData"),
        "same-module ref must use the disambiguated local class:\n{order_py}"
    );
    // The cross-namespace reference from a root type is disambiguated too.
    assert!(
        types_py.contains("ns: myapi.order.MyapiOrderInsertData"),
        "root cross-namespace ref must use the disambiguated name:\n{types_py}"
    );
    assert!(!types_py.contains("ns: myapi.order.OrderInsertData"));
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
fn test_python_split_modules_import_parent_for_top_level_refs() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct InsurerCategory {
        id: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct InsurerCategorySummary {
        name: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct CategoriesGetResponse {
        category: InsurerCategory,
        summaries: Vec<InsurerCategorySummary>,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct FormData {
        value: String,
        category: InsurerCategory,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct ConfigsInsertRequest {
        name: String,
        form_data: FormData,
    }

    async fn categories_get<S>(
        _s: S,
        _: reflectapi::Empty,
        _: reflectapi::Empty,
    ) -> CategoriesGetResponse {
        CategoriesGetResponse {
            category: InsurerCategory { id: String::new() },
            summaries: vec![InsurerCategorySummary {
                name: String::new(),
            }],
        }
    }

    async fn configs_insert<S>(
        _s: S,
        _request: ConfigsInsertRequest,
        _: reflectapi::Empty,
    ) -> reflectapi::Empty {
        reflectapi::Empty {}
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(categories_get, |b| b.name("offer_rules.categories.get"))
        .route(configs_insert, |b| b.name("offer_rules.configs.insert"))
        .rename_types(
            "reflectapi_demo::tests::namespace::InsurerCategory",
            "offer_rules::InsurerCategory",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::InsurerCategorySummary",
            "offer_rules::InsurerCategorySummary",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::CategoriesGetResponse",
            "offer_rules::categories::GetResponse",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::FormData",
            "offer_rules::model::input::FormData",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::ConfigsInsertRequest",
            "offer_rules::configs::InsertRequest",
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();

    let categories_file = files.get("offer_rules/categories/__init__.py").unwrap();
    assert!(categories_file.contains("import sys"), "{categories_file}");
    assert!(
        categories_file.contains("from ... import offer_rules"),
        "{categories_file}"
    );
    assert!(
        categories_file.contains("offer_rules.categories = sys.modules[__name__]"),
        "{categories_file}"
    );
    assert!(
        categories_file.contains("category: offer_rules.InsurerCategory"),
        "{categories_file}"
    );
    assert!(
        categories_file.contains("summaries: list[offer_rules.InsurerCategorySummary]"),
        "{categories_file}"
    );

    let configs_file = files.get("offer_rules/configs/__init__.py").unwrap();
    assert!(
        configs_file.contains("from ... import offer_rules"),
        "{configs_file}"
    );
    assert!(
        configs_file.contains("offer_rules.configs = sys.modules[__name__]"),
        "{configs_file}"
    );
    assert!(
        configs_file.contains("if not hasattr(offer_rules, \"model\"):"),
        "{configs_file}"
    );
    assert!(
        configs_file.contains("    offer_rules.model = _ReflectapiDeferredNamespace()"),
        "{configs_file}"
    );
    assert!(
        configs_file.contains("form_data: offer_rules.model.input.FormData"),
        "{configs_file}"
    );

    let form_data_file = files.get("offer_rules/model/input/__init__.py").unwrap();
    assert!(
        form_data_file.contains("from .... import offer_rules"),
        "{form_data_file}"
    );
    assert!(
        form_data_file.contains("offer_rules.model = sys.modules[__name__.rsplit(\".\", 1)[0]]"),
        "{form_data_file}"
    );
    assert!(
        form_data_file.contains("offer_rules.model.input = sys.modules[__name__]"),
        "{form_data_file}"
    );
    assert!(
        form_data_file.contains("category: offer_rules.InsurerCategory"),
        "{form_data_file}"
    );
}

#[test]
fn test_python_split_modules_order_sibling_imports_by_references() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Issue164Rule {
        id: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct Issue164Group {
        rule: Issue164Rule,
    }

    async fn group_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> Issue164Group {
        unimplemented!()
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(group_get, |b| b.name("commerce.group.get"))
        .rename_types(
            "reflectapi_demo::tests::namespace::Issue164Group",
            "commerce::group::Group",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::Issue164Rule",
            "commerce::rule::Rule",
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();

    let commerce_file = files.get("commerce/__init__.py").unwrap();
    let group_import = commerce_file.find("from . import group").unwrap();
    let rule_import = commerce_file.find("from . import rule").unwrap();
    assert!(
        rule_import < group_import,
        "`rule` must import before `group` because `group` references `commerce.rule`:\n{commerce_file}"
    );

    let group_file = files.get("commerce/group/__init__.py").unwrap();
    assert!(
        group_file.contains("rule: commerce.rule.Rule"),
        "{group_file}"
    );
}

#[test]
fn test_python_split_modules_handles_sys_root_and_sanitized_class_names() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct SysTop {
        value: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct SysUsesTop {
        top: SysTop,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct KeywordTop {
        value: String,
    }

    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct KeywordThing {
        top: KeywordTop,
    }

    async fn sys_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> SysUsesTop {
        SysUsesTop {
            top: SysTop {
                value: String::new(),
            },
        }
    }

    async fn keyword_get<S>(_s: S, _: reflectapi::Empty, _: reflectapi::Empty) -> KeywordThing {
        KeywordThing {
            top: KeywordTop {
                value: String::new(),
            },
        }
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(sys_get, |b| b.name("sys.child.get"))
        .route(keyword_get, |b| b.name("class.1-bad.for.get"))
        .rename_types("reflectapi_demo::tests::namespace::SysTop", "sys::Top")
        .rename_types(
            "reflectapi_demo::tests::namespace::SysUsesTop",
            "sys::child::UsesTop",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::KeywordTop",
            "class::Top",
        )
        .rename_types(
            "reflectapi_demo::tests::namespace::KeywordThing",
            "class::1-bad::for::Thing",
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();

    let sys_child_file = files.get("sys/child/__init__.py").unwrap();
    assert!(
        sys_child_file.contains("import sys as _reflectapi_sys"),
        "{sys_child_file}"
    );
    assert!(
        sys_child_file.contains("from ... import sys"),
        "{sys_child_file}"
    );
    assert!(
        sys_child_file.contains("sys.child = _reflectapi_sys.modules[__name__]"),
        "{sys_child_file}"
    );
    assert!(sys_child_file.contains("top: sys.Top"), "{sys_child_file}");

    let keyword_file = files.get("class_/_1_bad/for_/__init__.py").unwrap();
    assert!(
        keyword_file.contains("class Class_1BadForThing(BaseModel):"),
        "{keyword_file}"
    );
    assert!(
        keyword_file.contains("Thing = Class_1BadForThing"),
        "{keyword_file}"
    );
    assert!(
        !keyword_file.contains("Class1-badForThing"),
        "{keyword_file}"
    );

    let rebuild_file = files.get("_rebuild.py").unwrap();
    assert!(
        rebuild_file.contains("from .class_._1_bad.for_ import Class_1BadForThing"),
        "{rebuild_file}"
    );
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
fn test_python_split_modules_client_uses_hashed_namespace_class_name() {
    #[derive(
        Debug, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    struct LongRequest {
        value: String,
    }

    async fn long_get<S>(_s: S, request: LongRequest, _: reflectapi::Empty) -> LongRequest {
        request
    }

    let long_type_name = "tier1::GenericPredictionRequestVisionAnnotationBulkInputAnnotatedUnionVertexVisionAnnotationConfigStaticVisionAnnotationConfigRealtimeVisionAnnotationConfigBackgroundVisionAnnotationConfigRealtimeSyncVisionAnnotationConfigSymbolTextVisionAnnotationConfigRealtimeSupplierVisionAnnotationConfigLineBoxObbVisionAnnotationConfigPncocrVisionAnnotationConfigPncVisionAnnotationConfigStyleTransferPostProcessVisionAnnotationConfigStyleTransferLineBoxVisionAnnotationConfigFieldInfoAnnotationNoneTypeRequiredTrueDiscriminatorStrategy";
    let long_leaf_name = long_type_name.split("::").last().unwrap();

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(long_get, |b| b.name("tier1.long.get"))
        .rename_types(
            "reflectapi_demo::tests::namespace::LongRequest",
            long_type_name,
        )
        .build()
        .unwrap();

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();

    let tier1_file = files.get("tier1/__init__.py").unwrap();
    let class_line = tier1_file
        .lines()
        .find(|line| {
            line.starts_with("class Tier1GenericPredictionRequestVisionAnnotationBulkInput")
        })
        .unwrap();
    let class_name = class_line
        .strip_prefix("class ")
        .unwrap()
        .split('(')
        .next()
        .unwrap();
    let alias_name = class_name.strip_prefix("Tier1").unwrap();

    assert!(class_name.len() <= 80);
    assert_ne!(class_name, format!("Tier1{long_leaf_name}"));
    let alias_assignment = format!("{alias_name} = ");
    let alias_start = tier1_file.find(&alias_assignment).unwrap();
    let alias_block = tier1_file[alias_start..]
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(alias_block.contains(class_name), "{alias_block}");

    let client_file = files.get("_client.py").unwrap();
    assert!(client_file.contains(&format!("tier1.{alias_name}")));
    assert!(!client_file.contains(&format!("tier1.{long_leaf_name}")));
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
