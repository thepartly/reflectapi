#[macro_use]
mod assert;
mod basic;
mod enums;
mod generics;
mod namespace;
mod serde;

mod test_lib;

pub(crate) use assert::*;

#[test]
fn write_schema() {
    let (schema, _) = crate::builder().build().unwrap();

    std::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "reflectapi.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .unwrap();
}

#[test]
fn write_openapi_spec() {
    let (schema, _) = crate::builder().build().unwrap();

    let spec = reflectapi::codegen::openapi::Spec::from(&schema);
    let s = serde_json::to_string_pretty(&spec).unwrap();

    std::fs::write(
        format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "openapi.json"),
        &s,
    )
    .unwrap();
    insta::assert_snapshot!(s);
}
