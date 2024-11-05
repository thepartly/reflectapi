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
        &reflectapi::codegen::openapi::Config {
            exclude_tags: BTreeSet::from_iter(["internal".to_string()]),
            ..Default::default()
        },
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
        &reflectapi::codegen::rust::Config {
            format: true,
            instrument: true,
            typecheck: false,
            ..Default::default()
        },
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
    let src = reflectapi::codegen::typescript::generate(
        schema,
        &reflectapi::codegen::typescript::Config {
            format: true,
            typecheck: true,
            ..Default::default()
        },
    )
    .unwrap();

    std::fs::write(
        format!(
            "{}/clients/typescript/generated.ts",
            env!("CARGO_MANIFEST_DIR"),
        ),
        src,
    )
    .unwrap();
}
