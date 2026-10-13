#[macro_use]
mod assert;
mod basic;
mod enums;
mod generics;
mod namespace;
mod serde;

mod test_lib;

use std::collections::BTreeSet;

pub(crate) use assert::*;

#[test]
fn write_schema() {
    let (schema, _) = crate::builder().build().unwrap();

    std::fs::write(
        format!("{}/reflectapi.json", env!("CARGO_MANIFEST_DIR")),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .unwrap();
}

#[test]
fn write_openapi_spec() {
    let (schema, _) = crate::builder().build().unwrap();

    let s = reflectapi::codegen::openapi::generate(
        &schema,
        reflectapi::codegen::openapi::Config::default()
            .exclude_tags(BTreeSet::from_iter(["internal".to_string()])),
    )
    .unwrap();

    std::fs::write(format!("{}/openapi.json", env!("CARGO_MANIFEST_DIR")), &s).unwrap();
    insta::assert_snapshot!(s);
}

#[test]
fn generated_openapi_spec_has_resolvable_refs() {
    let (schema, _) = crate::builder().build().unwrap();

    let s = reflectapi::codegen::openapi::generate(
        &schema,
        reflectapi::codegen::openapi::Config::default()
            .exclude_tags(BTreeSet::from_iter(["internal".to_string()])),
    )
    .unwrap();
    let spec: serde_json::Value = serde_json::from_str(&s).unwrap();

    assert_no_embedded_json_schema_docs(&spec, "$".to_owned());
    assert_local_openapi_refs_resolve(&spec);
}

/// A generic instantiation that reaches itself only through a non-generic
/// enum: `Outcome<Observation>` is first converted from a route, then
/// re-entered from the `oneOf` of `Activation`, which `Observation` refers
/// to. The re-entry used to emit a `$ref` to a component never written.
#[test]
fn openapi_refs_resolve_for_generic_reentered_through_enum() {
    mod m {
        #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
        pub struct Outcome<O> {
            pub value: O,
        }

        #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
        pub struct Observation {
            pub activation: Option<Box<Activation>>,
        }

        #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
        pub enum Activation {
            Tuple(Outcome<Observation>),
            Struct { outcome: Outcome<Observation> },
            List(Vec<Outcome<Observation>>),
        }

        #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
        pub struct Request {
            pub outcome: Outcome<Observation>,
            pub activation: Activation,
        }
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(
            |_: (), _: m::Outcome<m::Observation>, _: reflectapi::Empty| async {
                reflectapi::Empty {}
            },
            |b| b.name("internal.outcome").tag("internal"),
        )
        .route(
            |_: (), r: m::Request, _: reflectapi::Empty| async { r },
            |b| b.name("public.activate").tag("public"),
        )
        .build()
        .unwrap();

    for exclude_tags in [BTreeSet::new(), BTreeSet::from(["internal".to_owned()])] {
        let spec = reflectapi::codegen::openapi::generate_spec(
            &schema,
            reflectapi::codegen::openapi::Config::default().exclude_tags(exclude_tags),
        );
        assert_local_openapi_refs_resolve(&serde_json::to_value(spec).unwrap());
    }
}

#[test]
fn openapi_refs_resolve_for_header_type_also_used_as_body() {
    #[derive(::serde::Deserialize, reflectapi::Input)]
    struct SharedHeaders {
        _authorization: String,
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(
            |_: (), _: SharedHeaders, _: reflectapi::Empty| async { reflectapi::Empty {} },
            |b| b.name("a.body"),
        )
        .route(
            |_: (), _: reflectapi::Empty, _: SharedHeaders| async { reflectapi::Empty {} },
            |b| b.name("b.headers"),
        )
        .build()
        .unwrap();
    assert_openapi_refs_resolve(&schema);
}

/// Generic header structs: one shared with a request body (plain and
/// self-referential), and a self-referential one used only as headers, whose
/// parameters refer back to its component.
#[test]
fn openapi_refs_resolve_for_generic_header_types() {
    #[derive(::serde::Deserialize, reflectapi::Input)]
    struct GenericHeaders<T> {
        _authorization: T,
    }

    #[derive(::serde::Deserialize, reflectapi::Input)]
    struct TreeHeaders<T> {
        _authorization: T,
        _children: Vec<TreeHeaders<T>>,
    }

    let (schema, _) = reflectapi::Builder::<()>::new()
        .route(
            |_: (), _: GenericHeaders<String>, _: reflectapi::Empty| async { reflectapi::Empty {} },
            |b| b.name("generic.body"),
        )
        .route(
            |_: (), _: reflectapi::Empty, _: GenericHeaders<String>| async { reflectapi::Empty {} },
            |b| b.name("generic.headers"),
        )
        .route(
            |_: (), _: TreeHeaders<String>, _: reflectapi::Empty| async { reflectapi::Empty {} },
            |b| b.name("tree.body"),
        )
        .route(
            |_: (), _: reflectapi::Empty, _: TreeHeaders<String>| async { reflectapi::Empty {} },
            |b| b.name("tree.headers"),
        )
        .route(
            |_: (), _: reflectapi::Empty, _: TreeHeaders<u8>| async { reflectapi::Empty {} },
            |b| b.name("tree.headers_only"),
        )
        .build()
        .unwrap();
    assert_openapi_refs_resolve(&schema);
}

pub(crate) fn assert_local_openapi_refs_resolve(spec: &serde_json::Value) {
    let schemas = spec
        .pointer("/components/schemas")
        .and_then(serde_json::Value::as_object)
        .expect("OpenAPI spec should have components.schemas");

    let mut unresolved_refs = Vec::new();
    collect_unresolved_local_refs(spec, schemas, "$".to_owned(), &mut unresolved_refs);

    assert!(
        unresolved_refs.is_empty(),
        "OpenAPI spec contains unresolved local refs:\n{}",
        unresolved_refs.join("\n")
    );
}

fn collect_unresolved_local_refs(
    value: &serde_json::Value,
    schemas: &serde_json::Map<String, serde_json::Value>,
    path: String,
    unresolved_refs: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(ref_path) = object.get("$ref").and_then(serde_json::Value::as_str) {
                if let Some(component_name) = ref_path.strip_prefix("#/components/schemas/") {
                    if !schemas.contains_key(component_name) {
                        unresolved_refs.push(format!("{path}: {ref_path}"));
                    }
                } else if ref_path.starts_with('#') {
                    unresolved_refs.push(format!("{path}: unsupported local ref {ref_path}"));
                }
            }

            for (key, child) in object {
                collect_unresolved_local_refs(
                    child,
                    schemas,
                    format!("{path}.{}", key.replace('.', "\\.")),
                    unresolved_refs,
                );
            }
        }
        serde_json::Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_unresolved_local_refs(
                    child,
                    schemas,
                    format!("{path}[{index}]"),
                    unresolved_refs,
                );
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}

fn assert_no_embedded_json_schema_docs(value: &serde_json::Value, path: String) {
    match value {
        serde_json::Value::String(text) => {
            assert!(
                !text.contains("<summary>JSON schema</summary>"),
                "OpenAPI spec contains embedded JSON schema docs at {path}"
            );
        }
        serde_json::Value::Object(object) => {
            for (key, child) in object {
                assert_no_embedded_json_schema_docs(
                    child,
                    format!("{path}.{}", key.replace('.', "\\.")),
                );
            }
        }
        serde_json::Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                assert_no_embedded_json_schema_docs(child, format!("{path}[{index}]"));
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

#[test]
fn write_rust_client() {
    let (schema, _) = crate::builder().build().unwrap();
    let src = reflectapi::codegen::rust::generate(
        schema,
        reflectapi::codegen::rust::Config::default()
            .format(true)
            .instrument(true),
    )
    .unwrap();

    std::fs::write(
        format!(
            "{}/clients/rust/generated/src/generated.rs",
            env!("CARGO_MANIFEST_DIR"),
        ),
        src,
    )
    .unwrap();
}

#[test]
fn instrumented_rust_client_does_not_record_request_bodies() {
    // Request bodies can carry credentials (passwords, tokens); recording
    // them as span fields would print them to logs in cleartext via their
    // `Debug` impl. Every generated `#[tracing::instrument]` attribute must
    // therefore skip all function arguments.
    let (schema, _) = crate::builder().build().unwrap();
    let src = reflectapi::codegen::rust::generate(
        schema,
        reflectapi::codegen::rust::Config::default()
            .format(true)
            .instrument(true),
    )
    .unwrap();

    let instrument_attributes: Vec<&str> = src
        .lines()
        .filter(|line| line.contains("tracing::instrument"))
        .collect();
    assert!(
        !instrument_attributes.is_empty(),
        "expected instrumented codegen to emit #[tracing::instrument] attributes"
    );
    for attribute in instrument_attributes {
        assert!(
            attribute.contains("skip_all"),
            "instrument attribute records function arguments (request body \
             may contain credentials): {attribute}"
        );
    }
}

#[test]
fn write_typescript_client() {
    let (schema, _) = crate::builder().build().unwrap();
    let files = reflectapi::codegen::typescript::generate(
        schema,
        reflectapi::codegen::typescript::Config::default()
            .format(true)
            .typecheck(true),
    )
    .unwrap();

    let dir = format!("{}/clients/typescript", env!("CARGO_MANIFEST_DIR"));
    for (filename, content) in files {
        std::fs::write(format!("{dir}/{filename}"), content).unwrap();
    }
}

#[test]
fn write_python_client() {
    let (schema, _) = crate::builder().build().unwrap();
    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config::default(),
    )
    .unwrap();

    let client_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("clients/python/api_client");
    for (filename, src) in files {
        let path = client_dir.join(filename);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
}
