//! Directional field-presence fixtures shared by the generated SDK tests.

#[allow(deprecated)]
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
    pub struct Pair {
        pub left: u32,
        pub right: u32,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct OptionalFlat {
        pub id: u32,
        #[serde(flatten)]
        pub pair: Option<Pair>,
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

    fn deserialize_memo<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Memo, D::Error> {
        let text: Option<String> = serde::Deserialize::deserialize(deserializer)?;
        Ok(Memo {
            text: text.unwrap_or_default(),
        })
    }

    #[derive(
        Default, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
    )]
    pub struct Memo {
        pub text: String,
    }

    #[derive(Default, serde::Serialize, serde::Deserialize, reflectapi::Input)]
    pub struct Cursor {
        pub after: Option<String>,
    }

    #[derive(Default, serde::Serialize, serde::Deserialize, reflectapi::Input)]
    pub struct Target {
        pub id: u32,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input)]
    #[serde(tag = "type")]
    pub enum Context {
        Scoped {
            #[serde(default)]
            value: Target,
        },
    }

    #[derive(Default, serde::Serialize, serde::Deserialize, reflectapi::Input)]
    #[serde(untagged)]
    pub enum Either {
        #[default]
        Nothing,
        Some(u32),
    }

    #[derive(Default, serde::Serialize, serde::Deserialize, reflectapi::Input)]
    pub struct MaybeText(pub Option<String>);

    // Omittable fields whose types serde reads from null or not. Every field
    // has a server default, so each may be left out.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input)]
    pub struct Create {
        #[serde(default)]
        pub memo: Memo,
        #[serde(default)]
        pub page: Cursor,
        #[serde(default)]
        pub boxed_target: Box<Target>,
        #[serde(default)]
        pub either: Either,
        #[serde(default)]
        pub maybe: MaybeText,
        #[serde(default)]
        pub boxed: Box<Option<String>>,
        #[serde(default)]
        pub json: serde_json::Value,
        #[serde(default, deserialize_with = "deserialize_memo")]
        pub custom: Memo,
    }

    // The server's types implement `Default`; the generated client's don't.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Defaulted {
        #[serde(default)]
        pub memo: Memo,
        #[serde(default)]
        pub boxed: Box<Memo>,
        #[serde(default)]
        pub text: String,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Output)]
    pub struct Config {
        pub level: u32,
    }

    // Response-only: a newer server may have removed the deprecated fields.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Output)]
    pub struct Profile {
        pub id: u32,
        #[deprecated(note = "use id")]
        pub legacy_tags: Option<std::collections::BTreeMap<String, u32>>,
        #[deprecated(note = "use id")]
        pub legacy_code: String,
        #[deprecated(note = "use id")]
        pub legacy_config: Config,
        #[deprecated(note = "use id")]
        pub legacy_rank: std::num::NonZeroU32,
    }

    // A deprecated field typed by a type parameter, which may lack `Default`.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Output)]
    pub struct Envelope<T> {
        pub data: T,
        #[deprecated(note = "use data")]
        pub result: T,
    }

    // Used in both directions, so a deprecated field stays required.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Shared {
        pub id: u32,
        #[deprecated(note = "use id")]
        pub old: String,
    }

    // Both directions agree when the serializer also omits absent values.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Equal {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub value: Option<String>,
    }
}

async fn profile(_: (), _: reflectapi::Empty, _: reflectapi::Empty) -> fixtures::Profile {
    unimplemented!("schema-only route")
}

async fn envelope(_: (), _: reflectapi::Empty, _: reflectapi::Empty) -> fixtures::Envelope<u32> {
    unimplemented!("schema-only route")
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
            |_: (), input: fixtures::OptionalFlat, _: reflectapi::Empty| async { input },
            |b| b.name("fields.optional_flat"),
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
        .route(
            |_: (), input: fixtures::Defaulted, _: reflectapi::Empty| async { input },
            |b| b.name("fields.defaulted"),
        )
        .route(profile, |b| b.name("fields.profile"))
        .route(envelope, |b| b.name("fields.envelope"))
        .route(
            |_: (), input: fixtures::Shared, _: reflectapi::Empty| async { input },
            |b| b.name("fields.shared"),
        )
        .build()
        .unwrap()
        .0
}

// Omittable fields that serde may or may not read from null. Generated for
// Python only: the Rust client needs `Default` on these types, which a
// fixture with only serde derives doesn't provide.
fn null_contract_schema() -> reflectapi::Schema {
    reflectapi::Builder::<()>::new()
        .name("Null contract")
        .route(
            |_: (), _: fixtures::Create, _: reflectapi::Empty| async {},
            |b| b.name("fields.create"),
        )
        .route(
            |_: (), _: fixtures::Context, _: reflectapi::Empty| async {},
            |b| b.name("fields.context"),
        )
        .build()
        .unwrap()
        .0
}

#[test]
fn write_null_contract_client() {
    let python = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/field-presence/python/null_contract_client");
    if python.exists() {
        std::fs::remove_dir_all(&python).unwrap();
    }
    for (filename, source) in reflectapi::codegen::python::generate_files(
        null_contract_schema(),
        &reflectapi::codegen::python::Config {
            package_name: "null_contract_client".into(),
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

#[test]
fn openapi_lets_responses_omit_deprecated_fields() {
    let spec = reflectapi::codegen::openapi::generate(&schema(), &Default::default()).unwrap();
    let spec: serde_json::Value = serde_json::from_str(&spec).unwrap();
    let required = |suffix: &str| {
        let (_, definition) = spec["components"]["schemas"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(name, _)| name.ends_with(suffix))
            .unwrap();
        definition["required"].as_array().unwrap().clone()
    };
    assert_eq!(required(".Profile"), vec![serde_json::json!("id")]);
    assert!(required(".Shared").contains(&serde_json::json!("old")));
}
