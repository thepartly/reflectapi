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

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    #[serde(rename_all = "snake_case")]
    pub enum FallbackExternal {
        Known,
        #[serde(other)]
        Unknown,
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
    pub struct FlattenNamedEnvelope {
        pub id: u32,
        pub tag: String,
        pub raw: String,
        #[serde(flatten)]
        pub value: OpenInternal,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct NestedFlattenEnvelope {
        pub ordinary: String,
        #[serde(flatten)]
        pub inner: FlattenEnumEnvelope,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub enum ClosedExternal {
        Unit,
        Value { message: String },
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenClosedExternalEnvelope {
        pub id: u32,
        pub ordinary: OpenInternal,
        #[serde(flatten)]
        pub value: ClosedExternal,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct OptionalFlattenClosedExternalEnvelope {
        pub id: u32,
        pub ordinary: OpenInternal,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        pub value: Option<ClosedExternal>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenAdjacentEnvelope {
        pub id: u32,
        #[serde(flatten)]
        pub value: OpenAdjacent,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct OptionalFlattenAdjacentEnvelope {
        pub id: u32,
        #[serde(flatten, skip_serializing_if = "Option::is_none")]
        pub value: Option<OpenAdjacent>,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenFallbackEnvelope {
        pub id: u32,
        #[serde(flatten)]
        pub value: FallbackInternal,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenProtoFallbackEnvelope {
        pub id: u32,
        #[serde(rename = "__proto__")]
        pub properties: BTreeMap<String, u32>,
        #[serde(flatten)]
        pub value: FallbackInternal,
    }

    #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
    pub struct FlattenAdjacentFallbackEnvelope {
        pub id: u32,
        #[serde(flatten)]
        pub value: FallbackAdjacent,
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
            |_: (), input: fixtures::FlattenNamedEnvelope, _: reflectapi::Empty| async { input },
            |b| b.name("enums.flatten_named"),
        )
        .route(
            |_: (), input: fixtures::NestedFlattenEnvelope, _: reflectapi::Empty| async { input },
            |b| b.name("enums.nested_flatten"),
        )
        .route(
            |_: (), input: fixtures::FlattenClosedExternalEnvelope, _: reflectapi::Empty| async {
                input
            },
            |b| b.name("enums.flatten_closed_external"),
        )
        .route(
            |_: (),
             input: fixtures::OptionalFlattenClosedExternalEnvelope,
             _: reflectapi::Empty| async { input },
            |b| b.name("enums.optional_flatten_closed_external"),
        )
        .route(
            |_: (), input: fixtures::FlattenAdjacentEnvelope, _: reflectapi::Empty| async { input },
            |b| b.name("enums.flatten_adjacent"),
        )
        .route(
            |_: (), input: fixtures::OptionalFlattenAdjacentEnvelope, _: reflectapi::Empty| async {
                input
            },
            |b| b.name("enums.optional_flatten_adjacent"),
        )
        .route(
            |_: (), input: fixtures::FlattenFallbackEnvelope, _: reflectapi::Empty| async { input },
            |b| b.name("enums.flatten_fallback"),
        )
        .route(
            |_: (), input: fixtures::FlattenProtoFallbackEnvelope, _: reflectapi::Empty| async {
                input
            },
            |b| b.name("enums.flatten_proto_fallback"),
        )
        .route(
            |_: (), input: fixtures::FlattenAdjacentFallbackEnvelope, _: reflectapi::Empty| async {
                input
            },
            |b| b.name("enums.flatten_adjacent_fallback"),
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
            |_: (), input: fixtures::FallbackExternal, _: reflectapi::Empty| async { input },
            |b| b.name("enums.external_fallback"),
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
fn open_rust_enum_keeps_public_variant_and_field_deprecation() {
    let mut schema = schema();
    let name = schema
        .output_types
        .types()
        .find(|ty| ty.name().ends_with("OpenInternal"))
        .unwrap()
        .name()
        .to_owned();
    let reflectapi::Type::Enum(enm) = schema.output_types.get_type_mut(&name).unwrap() else {
        panic!("expected OpenInternal enum");
    };
    let dog = enm
        .variants
        .iter_mut()
        .find(|variant| variant.name == "Dog")
        .unwrap();
    dog.deprecation_note = Some("Use Bird instead".into());
    dog.deprecated_in = Some("2.3.0".into());
    dog.deprecated_since = Some("2026-10-01".into());
    dog.sunset = Some("2026-11-01".into());
    let breed = dog
        .fields
        .iter_mut()
        .find(|field| field.name == "breed")
        .unwrap();
    breed.deprecation_note = Some("Use \"breed_code\" instead".into());
    breed.deprecated_in = Some("2.4.0".into());
    breed.deprecated_since = Some("2026-10-05".into());
    breed.sunset = Some("2026-11-05".into());

    let generated = reflectapi::codegen::rust::generate(schema, &Default::default()).unwrap();
    let public_enum = &generated[generated.rfind("pub enum OpenInternal {").unwrap()..];
    let public_enum = public_enum
        .split("// reflectapi-open-impl-start")
        .next()
        .unwrap();
    assert!(public_enum.contains(concat!(
        "#[deprecated(note = \"Deprecated in 2.3.0, since 2026-10-01; ",
        "removal after 2026-11-01. Use Bird instead\")]\n    Dog"
    )));
    assert!(public_enum.contains(concat!(
        "#[deprecated(note = \"Deprecated in 2.4.0, since 2026-10-05; ",
        "removal after 2026-11-05. Use \\\"breed_code\\\" instead\")]\n    breed:"
    )));
}

#[test]
fn python_flattened_enum_keeps_release_and_date_deprecation() {
    let mut schema = schema();
    let envelope_name = schema
        .output_types
        .types()
        .find(|ty| ty.name().ends_with("FlattenEnumEnvelope"))
        .unwrap()
        .name()
        .to_owned();
    let reflectapi::Type::Struct(envelope) =
        schema.output_types.get_type_mut(&envelope_name).unwrap()
    else {
        panic!("expected FlattenEnumEnvelope struct");
    };
    let value = envelope
        .fields
        .iter_mut()
        .find(|field| field.name == "value")
        .unwrap();
    value.deprecation_note = Some("Use replacement instead".into());
    value.deprecated_in = Some("2.3.0".into());
    value.deprecated_since = Some("2026-10-01".into());
    value.sunset = Some("2026-11-01".into());

    let enum_name = schema
        .output_types
        .types()
        .find(|ty| ty.name().ends_with("OpenInternal"))
        .unwrap()
        .name()
        .to_owned();
    let reflectapi::Type::Enum(enm) = schema.output_types.get_type_mut(&enum_name).unwrap() else {
        panic!("expected OpenInternal enum");
    };
    let breed = enm
        .variants
        .iter_mut()
        .find(|variant| variant.name == "Dog")
        .unwrap()
        .fields
        .iter_mut()
        .find(|field| field.name == "breed")
        .unwrap();
    breed.deprecation_note = Some("Use breed_code instead".into());
    breed.deprecated_in = Some("2.4.0".into());
    breed.deprecated_since = Some("2026-10-05".into());
    breed.sunset = Some("2026-11-05".into());

    let files = reflectapi::codegen::python::generate_files(
        schema,
        &reflectapi::codegen::python::Config {
            package_name: "enum_client".into(),
            format: false,
            ..Default::default()
        },
    )
    .unwrap();
    let output = &files["enum_compatibility/fixtures/output/__init__.py"];
    assert!(output.lines().any(|line| {
        line.trim_start().starts_with("value: ")
            && line.contains("Deprecated in 2.3.0, since 2026-10-01; removal after 2026-11-01. Use replacement instead")
    }));
    assert!(output.lines().any(|line| {
        line.trim_start().starts_with("breed: str")
            && line.contains("Deprecated in 2.4.0, since 2026-10-05; removal after 2026-11-05. Use breed_code instead")
    }));
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
        serde_json::json!({"kind": 0, "message": "indexed"}),
        serde_json::json!({"kind": 2}),
        serde_json::json!({"kind": true}),
        serde_json::json!({"kind": null}),
        serde_json::json!({"kind": -1}),
        serde_json::json!({"kind": 1.5}),
        serde_json::json!({"data": {}}),
    ];
    let external_samples = [
        serde_json::json!("known"),
        serde_json::json!("unknown"),
        serde_json::json!("future"),
        serde_json::json!(""),
        serde_json::json!({"known": null}),
        serde_json::json!({"future": null}),
        serde_json::json!({"unknown": null}),
        serde_json::json!({"unknown": 5}),
        serde_json::json!({"known": 5}),
        serde_json::json!({"future": 5}),
        serde_json::json!({"future": {}}),
        serde_json::json!({"known": null, "future": null}),
        serde_json::json!({}),
        serde_json::json!(5),
        serde_json::json!(null),
        serde_json::json!(true),
    ];
    let flattened_samples = samples
        .iter()
        .map(|sample| {
            let mut wire = sample.clone();
            wire.as_object_mut()
                .unwrap()
                .insert("id".into(), serde_json::json!(7));
            wire
        })
        .collect::<Vec<_>>();
    std::fs::write(
        root.join("fallback_samples.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "internal": fallback_samples::<fixtures::FallbackInternal>(&samples),
            "adjacent": fallback_samples::<fixtures::FallbackAdjacent>(&samples),
            "external": fallback_samples::<fixtures::FallbackExternal>(&external_samples),
            "flatten_internal": fallback_samples::<fixtures::FlattenFallbackEnvelope>(&flattened_samples),
            "flatten_adjacent": fallback_samples::<fixtures::FlattenAdjacentFallbackEnvelope>(&flattened_samples),
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
