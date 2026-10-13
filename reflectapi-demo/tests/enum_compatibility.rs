//! Fixed generated clients tested against newer wire values in all three SDKs.
//! The integration suites consume `target/enum-compatibility/` under this crate.

pub mod fixtures {
    use std::collections::BTreeMap;

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "kind")]
    #[reflectapi(open, derive(serde::Serialize, serde::Deserialize))]
    pub enum OpenInternal {
        Dog { breed: String },
        Bird,
    }

    impl reflectapi::StatusCode for OpenInternal {
        fn status_code(&self) -> http::StatusCode {
            http::StatusCode::BAD_REQUEST
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "kind", content = "data")]
    #[reflectapi(open)]
    pub enum OpenAdjacent {
        Value { message: String },
        Unit,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[reflectapi(open)]
    pub enum OpenExternal {
        Unit,
        Value { message: String },
        Number(u32),
        Tuple(u32, String),
        Optional(Option<String>),
    }

    // A real Unknown variant must not collide with the generated unknown case.
    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[reflectapi(open)]
    pub enum OpenUnit {
        Ready,
        Unknown,
    }

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct ResponseHeaders {
        #[serde(rename = "x-kind")]
        pub kind: Option<OpenUnit>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "kind")]
    #[reflectapi(open, derive(serde::Serialize, serde::Deserialize))]
    pub enum OpenGeneric<T> {
        Value { value: T },
        Unit,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "kind")]
    #[reflectapi(open, derive(serde::Serialize, serde::Deserialize))]
    pub enum OpenGenericDS<D, S> {
        Value { first: D, second: S },
        Unit,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "kind")]
    pub enum FallbackInternal {
        Value {
            message: String,
        },
        #[serde(other)]
        #[serde(rename = "other")]
        Other,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "kind", content = "data")]
    pub enum FallbackAdjacent {
        Value {
            message: String,
        },
        #[serde(other)]
        Other,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Output)]
    #[serde(tag = "kind")]
    #[reflectapi(open, derive(serde_derive::Serialize, serde_alias::Deserialize))]
    pub enum OpenAlternateDerives {
        Value { value: String },
        Unit,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(transparent)]
    pub struct OpenAlias(pub OpenInternal);

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenPartA {
        pub first: OpenUnit,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenPartB {
        pub second: OpenInternal,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenEnvelope {
        pub ordinary: OpenUnit,
        #[serde(flatten)]
        pub first_part: FlattenPartA,
        #[serde(flatten)]
        pub second_part: FlattenPartB,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenEnumEnvelope {
        pub id: u32,
        #[serde(flatten)]
        pub value: OpenInternal,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct OptionalFlattenEnumEnvelope {
        pub id: u32,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        pub value: Option<OpenInternal>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(untagged)]
    pub enum UntaggedWrapper {
        Tagged(OpenInternal),
        Text(String),
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(tag = "kind")]
    #[reflectapi(open)]
    pub enum OpenNested {
        Wrapped { value: UntaggedWrapper },
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct Envelope {
        pub internal: OpenInternal,
        pub adjacent: OpenAdjacent,
        pub external: OpenExternal,
        pub units: OpenUnit,
        pub generic: OpenGeneric<String>,
        pub fallback: FallbackInternal,
        pub adjacent_fallback: FallbackAdjacent,
        pub optional: Option<OpenInternal>,
        pub list: Vec<OpenInternal>,
        pub map: BTreeMap<String, OpenInternal>,
        pub alias: OpenAlias,
    }
}

fn schema() -> reflectapi::Schema {
    reflectapi::Builder::<()>::new()
        .name("Enum compatibility")
        .response_headers::<fixtures::ResponseHeaders>()
        .route(
            |_: (), input: fixtures::Envelope, _: reflectapi::Empty| async { input },
            |b| b.name("enums.echo"),
        )
        .route(
            |_: (), input: fixtures::OpenInternal, _: reflectapi::Empty| async { input },
            |b| b.name("enums.internal"),
        )
        .route(
            |_: (), input: fixtures::OpenAdjacent, _: reflectapi::Empty| async { input },
            |b| b.name("enums.adjacent"),
        )
        .route(
            |_: (), input: fixtures::OpenExternal, _: reflectapi::Empty| async { input },
            |b| b.name("enums.external"),
        )
        .route(
            |_: (), input: fixtures::OpenUnit, _: reflectapi::Empty| async { input },
            |b| b.name("enums.units"),
        )
        .route(
            |_: (), _: reflectapi::Empty, _: reflectapi::Empty| async {
                fixtures::OpenAlternateDerives::Unit
            },
            |b| b.name("enums.alternate_derives"),
        )
        .route(
            |_: (), input: fixtures::OpenGenericDS<String, u32>, _: reflectapi::Empty| async {
                input
            },
            |b| b.name("enums.generic_ds"),
        )
        .route(
            |_: (), input: fixtures::FlattenEnvelope, _: reflectapi::Empty| async { input },
            |b| b.name("enums.flatten"),
        )
        .route(
            |_: (), input: fixtures::FlattenEnumEnvelope, _: reflectapi::Empty| async { input },
            |b| b.name("enums.flatten_enum"),
        )
        .route(
            |_: (), input: fixtures::OptionalFlattenEnumEnvelope, _: reflectapi::Empty| async {
                input
            },
            |b| b.name("enums.optional_flatten_enum"),
        )
        .route(
            |_: (), input: fixtures::UntaggedWrapper, _: reflectapi::Empty| async { input },
            |b| b.name("enums.untagged"),
        )
        .route(
            |_: (), input: fixtures::OpenNested, _: reflectapi::Empty| async { input },
            |b| b.name("enums.nested"),
        )
        .route(
            |_: (), input: fixtures::FallbackInternal, _: reflectapi::Empty| async { input },
            |b| b.name("enums.fallback"),
        )
        .route(
            |_: (), input: fixtures::FallbackAdjacent, _: reflectapi::Empty| async { input },
            |b| b.name("enums.adjacent_fallback"),
        )
        .route(
            |_: (), _: reflectapi::Empty, _: reflectapi::Empty| async {
                Err::<reflectapi::Empty, fixtures::OpenInternal>(fixtures::OpenInternal::Bird)
            },
            |b| b.name("enums.error"),
        )
        .stream_route(
            |_: (), _: reflectapi::Empty, _: reflectapi::Empty| {
                Ok::<_, reflectapi::Infallible>(futures_util::stream::iter([
                    fixtures::OpenInternal::Bird,
                ]))
            },
            |b| b.name("enums.stream"),
        )
        .build()
        .unwrap()
        .0
}

#[test]
fn response_policies_do_not_widen_requests() {
    let schema = schema();
    let reflected_input = schema
        .input_types
        .types()
        .filter_map(|ty| match ty {
            reflectapi::Type::Enum(enm) if enm.name.contains("fixtures") => Some(enm),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!reflected_input.is_empty());
    assert!(reflected_input.iter().all(|enm| !enm.open));
    assert!(reflected_input
        .iter()
        .flat_map(|enm| &enm.variants)
        .all(|variant| !variant.other));
    assert!(schema.output_types.types().any(|ty| matches!(ty,
        reflectapi::Type::Enum(enm) if enm.name.ends_with("OpenInternal") && enm.open)));
    assert!(schema.output_types.types().any(|ty| matches!(ty,
        reflectapi::Type::Enum(enm) if enm.name.ends_with("FallbackInternal")
            && enm.variants.iter().any(|variant| variant.other))));
    let roundtrip: reflectapi::Schema =
        serde_json::from_value(serde_json::to_value(&schema).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&schema).unwrap(),
        serde_json::to_value(&roundtrip).unwrap()
    );
}

#[test]
fn old_enum_schemas_keep_closed_defaults() {
    let enm: reflectapi::Enum = serde_json::from_value(serde_json::json!({
        "name": "State", "variants": [{"name": "Ready", "fields": "none"}]
    }))
    .unwrap();
    assert!(!enm.open);
    assert!(!enm.variants[0].other);
    let json = serde_json::to_value(&enm).unwrap();
    assert!(json.get("open").is_none());
    assert!(json["variants"][0].get("other").is_none());
}

#[test]
fn write_generated_clients() {
    let schema = schema();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/enum-compatibility");
    std::fs::create_dir_all(root.join("rust")).unwrap();
    std::fs::write(
        root.join("schema.json"),
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .unwrap();
    // Serde itself is the oracle for the declared unit fallback. In particular,
    // adjacent content must obey the original unit variant's decoding rules.
    fn fallback_samples<T: serde::de::DeserializeOwned + serde::Serialize>(
        samples: &[serde_json::Value],
    ) -> Vec<serde_json::Value> {
        samples
            .iter()
            .map(|wire| match serde_json::from_value::<T>(wire.clone()) {
                Ok(value) => serde_json::json!({"wire": wire, "ok": true, "value": value}),
                Err(error) => {
                    serde_json::json!({"wire": wire, "ok": false, "error": error.to_string()})
                }
            })
            .collect()
    }
    let samples = [
        serde_json::json!({"kind": "Future"}),
        serde_json::json!({"kind": "Future", "data": null}),
        serde_json::json!({"kind": "Future", "data": {"new": true}}),
        serde_json::json!({"kind": "Future", "data": 5}),
        serde_json::json!({"kind": "Other"}),
        serde_json::json!({"kind": "Other", "data": {}}),
        serde_json::json!({"kind": "other", "new": true}),
        serde_json::json!({"kind": "Value", "data": {"message": "known"}, "message": "known"}),
        serde_json::json!({"kind": "Value", "data": {}}),
        serde_json::json!({"kind": 1}),
        serde_json::json!({"data": {}}),
    ];
    std::fs::write(
        root.join("fallback_samples.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "internal": fallback_samples::<fixtures::FallbackInternal>(&samples),
            "adjacent": fallback_samples::<fixtures::FallbackAdjacent>(&samples),
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("rust/generated.rs"),
        reflectapi::codegen::rust::generate(schema.clone(), &Default::default()).unwrap(),
    )
    .unwrap();
    for (filename, source) in
        reflectapi::codegen::typescript::generate(schema.clone(), &Default::default()).unwrap()
    {
        let path = root.join("typescript").join(filename);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let python = root.join("python/enum_client");
    if python.exists() {
        std::fs::remove_dir_all(&python).unwrap();
    }
    for (filename, source) in reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config {
            package_name: "enum_client".into(),
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
