use std::collections::BTreeSet;

fn required_headers() -> BTreeSet<String> {
    BTreeSet::from_iter(["X-Api-Key".to_string(), "x-tenant-id".to_string()])
}

fn schema() -> reflectapi::Schema {
    crate::builder().build().unwrap().0
}

#[test]
fn rust_client_takes_required_headers_at_construction() {
    let src = reflectapi::codegen::rust::generate(
        schema(),
        reflectapi::codegen::rust::Config::default()
            .format(true)
            .typecheck(std::env::var("CI").is_ok())
            .required_headers(required_headers()),
    )
    .unwrap();

    assert!(src.contains("pub use interface::RequiredHeaders;"), "{src}");
    assert!(src.contains("pub struct RequiredHeaders"), "{src}");
    assert!(
        src.contains("pub x_api_key: reflectapi::rt::HeaderValue"),
        "{src}"
    );
    assert!(
        src.contains("pub x_tenant_id: reflectapi::rt::HeaderValue"),
        "{src}"
    );
    assert!(
        src.contains("HeaderName::from_static(\"x-api-key\")"),
        "{src}"
    );
    assert!(
        src.contains("pub fn new(client: C, required_headers: RequiredHeaders) -> Self"),
        "{src}"
    );

    assert!(
        src.contains(
            "pub fn new(\n            x_api_key: reflectapi::rt::HeaderValue,\n            x_tenant_id: reflectapi::rt::HeaderValue,\n        ) -> Self"
        ),
        "{src}"
    );

    assert_eq!(
        src.matches("required_headers: RequiredHeaders").count(),
        2,
        "expected `new` and `try_new` to be the only constructors taking headers:\n{src}"
    );
}

#[test]
fn rust_client_without_required_headers_is_unchanged() {
    let src = reflectapi::codegen::rust::generate(
        schema(),
        reflectapi::codegen::rust::Config::default().format(true),
    )
    .unwrap();

    assert!(!src.contains("RequiredHeaders"), "{src}");
    assert!(src.contains("pub fn new(client: C) -> Self"), "{src}");
}

#[test]
fn typescript_client_takes_required_headers_at_construction() {
    let files = reflectapi::codegen::typescript::generate(
        schema(),
        reflectapi::codegen::typescript::Config::default()
            .format(true)
            .typecheck(true)
            .required_headers(required_headers()),
    )
    .unwrap();
    let src = &files["generated.ts"];

    assert!(src.contains("export type RequiredHeaders = {"), "{src}");
    assert!(src.contains("\"x-api-key\": string;"), "{src}");
    assert!(src.contains("\"x-tenant-id\": string;"), "{src}");
    assert!(src.contains("required_headers: RequiredHeaders"), "{src}");
    assert!(src.contains("__with_required_headers("), "{src}");
}

#[test]
fn typescript_client_without_required_headers_is_unchanged() {
    let files = reflectapi::codegen::typescript::generate(
        schema(),
        reflectapi::codegen::typescript::Config::default().format(true),
    )
    .unwrap();
    let src = &files["generated.ts"];

    assert!(!src.contains("RequiredHeaders"), "{src}");
    assert!(
        src.contains("export function client(base: string | Client)"),
        "{src}"
    );
}

#[test]
fn python_client_takes_required_headers_as_keyword_arguments() {
    let config = reflectapi::codegen::python::Config {
        generate_sync: true,
        required_headers: required_headers(),
        ..Default::default()
    };
    let src = reflectapi::codegen::python::generate(schema(), &config).unwrap();

    assert_eq!(src.matches("        x_api_key: str,").count(), 2, "{src}");
    assert_eq!(src.matches("        x_tenant_id: str,").count(), 2, "{src}");
    assert_eq!(
        src.matches("            \"x-api-key\": x_api_key,").count(),
        2,
        "{src}"
    );
    assert!(
        src.contains("SyncRequiredHeadersMiddleware(required_headers)"),
        "{src}"
    );
    assert!(
        src.contains("AsyncRequiredHeadersMiddleware(required_headers)"),
        "{src}"
    );
    assert_eq!(
        src.matches("*(kwargs.pop(\"middleware\", None) or []),")
            .count(),
        2,
        "{src}"
    );
}

#[test]
fn python_client_without_required_headers_is_unchanged() {
    let config = reflectapi::codegen::python::Config {
        generate_sync: true,
        ..Default::default()
    };
    let src = reflectapi::codegen::python::generate(schema(), &config).unwrap();

    assert!(!src.contains("x_api_key"), "{src}");
    assert!(!src.contains("RequiredHeadersMiddleware"), "{src}");
    assert!(
        src.contains("super().__init__(base_url, **kwargs)"),
        "{src}"
    );
}

#[test]
fn openapi_marks_required_headers_as_required_parameters() {
    let spec = reflectapi::codegen::openapi::generate(
        &schema(),
        reflectapi::codegen::openapi::Config::default().required_headers(required_headers()),
    )
    .unwrap();
    let spec: serde_json::Value = serde_json::from_str(&spec).unwrap();

    let paths = spec["paths"].as_object().unwrap();
    assert!(!paths.is_empty());
    for (path, item) in paths {
        let parameters = item["post"]["parameters"].as_array().unwrap();
        for name in ["x-api-key", "x-tenant-id"] {
            let parameter = parameters
                .iter()
                .find(|p| p["name"] == name)
                .unwrap_or_else(|| panic!("{path} is missing the `{name}` parameter"));
            assert_eq!(parameter["in"], "header", "{path}");
            assert_eq!(parameter["required"], true, "{path}");
            assert_eq!(parameter["schema"]["type"], "string", "{path}");
        }
    }
}

#[test]
fn openapi_does_not_duplicate_a_header_the_handler_declares() {
    let spec = reflectapi::codegen::openapi::generate(
        &schema(),
        reflectapi::codegen::openapi::Config::default()
            .required_headers(BTreeSet::from_iter(["authorization".to_string()])),
    )
    .unwrap();
    let spec: serde_json::Value = serde_json::from_str(&spec).unwrap();

    let parameters = spec["paths"]["/pets.list"]["post"]["parameters"]
        .as_array()
        .unwrap();
    let matching = parameters
        .iter()
        .filter(|p| p["name"] == "authorization")
        .count();
    assert_eq!(matching, 1, "{parameters:#?}");
}

#[test]
fn invalid_required_header_name_is_rejected() {
    let err = reflectapi::codegen::openapi::generate(
        &schema(),
        reflectapi::codegen::openapi::Config::default()
            .required_headers(BTreeSet::from_iter(["x api key".to_string()])),
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("required_headers"), "{err}");
}
