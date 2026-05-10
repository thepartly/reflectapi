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

    let client_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("clients/python");
    for (filename, src) in files {
        let path = client_dir.join(filename);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
}
