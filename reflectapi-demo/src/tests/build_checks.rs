//! The checks `build()` always applies, for schemas whose clients would be
//! wrong.

fn build_errors(builder: reflectapi::Builder<()>) -> Vec<String> {
    match builder.build() {
        Ok(_) => Vec::new(),
        Err(errors) => errors.0.iter().map(ToString::to_string).collect(),
    }
}

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
#[serde(untagged)]
enum TwoNulls {
    Archived,
    Deleted,
    Named(String),
}

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
enum OneNull {
    Known,
    #[serde(untagged)]
    Unknown,
}

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
struct Holder {
    two: TwoNulls,
    alone: OneNull,
    optional: Option<OneNull>,
    nested: Vec<reflectapi::Option<OneNull>>,
    double: Option<Option<String>>,
}

async fn hold(_: (), holder: Holder, _: reflectapi::Empty) -> Holder {
    holder
}

#[test]
fn untagged_units_that_lose_data_are_refused() {
    let errors = build_errors(reflectapi::Builder::new().route(hold, |b| b.name("hold")));
    let holder = "reflectapi_demo::tests::build_checks::Holder";
    let one_null = "reflectapi_demo::tests::build_checks::OneNull";
    assert_eq!(
        errors,
        [
            format!("`{holder}.nested` is `reflectapi::Option<{one_null}>`, and `{one_null}::Unknown` is an untagged unit, written as `null`, which reads back as `None`"),
            format!("`{holder}.optional` is `std::option::Option<{one_null}>`, and `{one_null}::Unknown` is an untagged unit, written as `null`, which reads back as `None`"),
            "`reflectapi_demo::tests::build_checks::TwoNulls`: variants `Archived`, `Deleted` are untagged units, so each is written as `null`, and `null` reads back as `Archived`".to_string(),
        ]
    );
}

#[derive(serde::Serialize, reflectapi::Output)]
struct Leaf {
    id: u32,
}

#[derive(serde::Serialize, reflectapi::Output)]
struct Root {
    leaf: Leaf,
}

async fn root(_: (), _: reflectapi::Empty, _: reflectapi::Empty) -> Root {
    Root {
        leaf: Leaf { id: 1 },
    }
}

#[test]
fn a_reference_through_a_shadowed_module_is_refused() {
    // Inside `a::b`, the clients read `a::…` as `a::b::a::…`.
    let errors = build_errors(
        reflectapi::Builder::new()
            .route(root, |b| b.name("root"))
            .rename_types(
                "reflectapi_demo::tests::build_checks::Leaf",
                "a::b::a::Leaf",
            )
            .rename_types("reflectapi_demo::tests::build_checks::Root", "a::b::Root"),
    );
    assert_eq!(
        errors,
        ["`a::b::Root` refers to `a::b::a::Leaf`, but inside `a::b` the TypeScript and Python clients read `a::…` as `a::b::a::…`; rename one of the modules with `rename_types`"]
    );
}

#[test]
fn a_module_named_like_a_root_is_fine_when_nothing_crosses_it() {
    // `c::a` shadows `a` inside `c`, but nothing in `c` refers to `a`.
    let errors = build_errors(
        reflectapi::Builder::new()
            .route(root, |b| b.name("root"))
            .rename_types("reflectapi_demo::tests::build_checks::Leaf", "c::a::Leaf")
            .rename_types("reflectapi_demo::tests::build_checks::Root", "a::Root"),
    );
    assert_eq!(errors, Vec::<String>::new());
}
