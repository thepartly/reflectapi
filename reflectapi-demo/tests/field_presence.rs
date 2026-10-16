//! Directional field-presence fixtures shared by the generated SDK tests.

pub mod fixtures {
    fn deserialize_nullable<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<String>, D::Error> {
        serde::Deserialize::deserialize(deserializer)
    }

    fn serialize_nullable<S: serde::Serializer>(
        value: &Option<String>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(value, serializer)
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Fields {
        pub plain: Option<String>,
        #[serde(deserialize_with = "deserialize_nullable")]
        pub custom: Option<String>,
        #[serde(default, deserialize_with = "deserialize_nullable")]
        pub defaulted: Option<String>,
        #[serde(serialize_with = "serialize_nullable")]
        pub serializer_only: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub skipped: Option<String>,
        pub three_state: reflectapi::Option<String>,
        #[serde(default)]
        pub default_three_state: reflectapi::Option<String>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "kind")]
    pub enum Variant {
        Value { value: Option<String> },
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Flat {
        pub id: u32,
        #[serde(flatten)]
        pub fields: Fields,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Wrapper<T> {
        pub value: Option<T>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Nested {
        pub one: Wrapper<String>,
        pub many: Vec<Wrapper<String>>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Tuple(pub Option<String>, pub Option<String>);

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Defaults {
        #[serde(default)]
        pub count: u32,
    }

    // Both directions agree when the serializer also omits absent values.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Equal {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<String>,
    }
}

fn schema() -> reflectapi::Schema {
    reflectapi::Builder::<()>::new()
        .name("Field presence")
        .route(
            |_: (), input: fixtures::Fields, _: reflectapi::Empty| async { input },
            |b| b.name("fields.echo"),
        )
        .route(
            |_: (), input: fixtures::Variant, _: reflectapi::Empty| async { input },
            |b| b.name("fields.variant"),
        )
        .route(
            |_: (), input: fixtures::Flat, _: reflectapi::Empty| async { input },
            |b| b.name("fields.flat"),
        )
        .route(
            |_: (), input: fixtures::Wrapper<String>, _: reflectapi::Empty| async { input },
            |b| b.name("fields.generic"),
        )
        .route(
            |_: (), input: fixtures::Equal, _: reflectapi::Empty| async { input },
            |b| b.name("fields.equal"),
        )
        .route(
            |_: (), input: fixtures::Nested, _: reflectapi::Empty| async { input },
            |b| b.name("fields.nested"),
        )
        .route(
            |_: (), input: fixtures::Tuple, _: reflectapi::Empty| async { input },
            |b| b.name("fields.tuple"),
        )
        .route(
            |_: (), input: fixtures::Defaults, _: reflectapi::Empty| async { input },
            |b| b.name("fields.defaults"),
        )
        .build()
        .unwrap()
        .0
}

#[test]
fn consolidation_preserves_directional_presence() {
    let mut schema = schema();
    let types = schema.consolidate_types();
    for name in ["Fields", "Variant", "Flat", "Wrapper"] {
        assert!(
            types
                .iter()
                .any(|ty| ty.ends_with(&format!("::input::{name}"))),
            "{types:?}"
        );
        assert!(
            types
                .iter()
                .any(|ty| ty.ends_with(&format!("::output::{name}"))),
            "{types:?}"
        );
    }
    assert!(types.iter().any(|ty| ty.ends_with("::fixtures::Equal")));
    assert!(!types.iter().any(|ty| ty.ends_with("::input::Equal")));
}

#[test]
fn explicit_schema_requiredness_is_authoritative() {
    let mut schema = schema();
    let name = schema
        .input_types
        .types()
        .find(|ty| ty.name().ends_with("::Fields"))
        .unwrap()
        .name()
        .to_owned();
    let reflectapi::Type::Struct(fields) = schema.input_types.get_type_mut(&name).unwrap() else {
        panic!("expected struct");
    };
    fields
        .fields
        .iter_mut()
        .find(|field| field.name == "plain")
        .unwrap()
        .required = true;
    // Replaying a persisted schema must not infer optionality from its type name.
    let schema: reflectapi::Schema =
        serde_json::from_str(&serde_json::to_string(&schema).unwrap()).unwrap();
    let files =
        reflectapi::codegen::typescript::generate(schema.clone(), &Default::default()).unwrap();
    let generated = files
        .iter()
        .find(|(name, _)| name.as_str() == "generated.ts")
        .unwrap()
        .1
        .as_str();
    assert!(!generated
        .lines()
        .any(|line| line.trim_start().starts_with("plain?:")));
    assert!(generated
        .lines()
        .any(|line| line.trim_start().starts_with("plain:")));
    let python = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config {
            format: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!python
        .values()
        .any(|source| source.contains("plain: str | None = None")));
}

#[test]
fn write_generated_clients() {
    let schema = schema();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/field-presence");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("schema.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .unwrap();
    for (filename, source) in
        reflectapi::codegen::typescript::generate(schema.clone(), &Default::default()).unwrap()
    {
        let path = root.join("typescript").join(filename);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    std::fs::create_dir_all(root.join("rust")).unwrap();
    let mut rust_config = reflectapi::codegen::rust::Config::default();
    rust_config.base_derives(std::collections::BTreeSet::from_iter([
        "Debug".into(),
        "serde::Serialize".into(),
        "serde::Deserialize".into(),
    ]));
    std::fs::write(
        root.join("rust/generated.rs"),
        reflectapi::codegen::rust::generate(schema.clone(), &rust_config).unwrap(),
    )
    .unwrap();
    let python = root.join("python/presence_client");
    if python.exists() {
        std::fs::remove_dir_all(&python).unwrap();
    }
    for (filename, source) in reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config {
            package_name: "presence_client".into(),
            generate_async: true,
            generate_sync: true,
            generate_testing: false,
            format: false,
            base_url: None,
        },
    )
    .unwrap()
    {
        let path = python.join(filename);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
}

#[test]
fn openapi_uses_the_same_directional_required_keys() {
    let spec = reflectapi::codegen::openapi::generate(&schema(), &Default::default()).unwrap();
    let spec: serde_json::Value = serde_json::from_str(&spec).unwrap();
    let definitions = spec["components"]["schemas"].as_object().unwrap();
    let fields = definitions
        .values()
        .filter(|definition| {
            definition["properties"].get("plain").is_some()
                && definition["properties"].get("custom").is_some()
                && definition["properties"].get("id").is_none()
        })
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 2);
    let required =
        |definition: &serde_json::Value| definition["required"].as_array().unwrap().clone();
    assert!(fields
        .iter()
        .any(|definition| required(definition) == vec![serde_json::json!("custom")]));
    assert!(fields.iter().any(|definition| {
        let keys = required(definition);
        [
            "plain",
            "custom",
            "defaulted",
            "serializer_only",
            "three_state",
            "default_three_state",
        ]
        .iter()
        .all(|name| keys.contains(&serde_json::json!(name)))
            && !keys.contains(&serde_json::json!("skipped"))
    }));
}
