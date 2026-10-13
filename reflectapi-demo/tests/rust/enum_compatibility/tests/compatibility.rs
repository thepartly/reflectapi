use reflectapi_enum_compatibility_rust::types::enum_compatibility::fixtures::output::{
    FallbackAdjacent, FallbackInternal, FlattenEnumEnvelope, FlattenEnvelope, OpenAdjacent,
    OpenExternal, OpenGeneric, OpenGenericDS, OpenInternal, OpenNested, OpenUnit, OpenWithFallback,
    OptionalFlattenEnumEnvelope, UntaggedWrapper,
};
use serde_json::{json, Value};

fn decode<T: serde::de::DeserializeOwned>(raw: Value) -> T {
    serde_json::from_value(raw).unwrap()
}

#[test]
fn internal_preserves_unknown_and_validates_known() {
    let raw = json!({"kind": "Fox", "tail": [1, 2], "active": true});
    match decode::<OpenInternal>(raw.clone()) {
        OpenInternal::Unknown { tag, raw: retained } => {
            assert_eq!(tag, "Fox");
            assert_eq!(retained, raw);
        }
        _ => panic!("expected unknown variant"),
    }
    assert!(matches!(
        decode::<OpenInternal>(json!({"kind": "Bird"})),
        OpenInternal::Bird
    ));
    assert!(
        matches!(decode::<OpenInternal>(json!({"kind": "Dog", "breed": "Collie"})), OpenInternal::Dog { breed } if breed == "Collie")
    );
    for raw in [
        json!({"kind": "Dog", "breed": 4}),
        json!({"kind": "Dog"}),
        json!({"breed": "Collie"}),
        json!({"kind": 4}),
    ] {
        assert!(serde_json::from_value::<OpenInternal>(raw).is_err());
    }
}

#[test]
fn adjacent_preserves_unknown_and_validates_known() {
    let raw = json!({"kind": "Future", "data": {"v": 2}, "extra": true});
    assert!(
        matches!(decode::<OpenAdjacent>(raw.clone()), OpenAdjacent::Unknown { tag, raw: retained } if tag == "Future" && retained == raw)
    );
    assert!(matches!(
        decode::<OpenAdjacent>(json!({"kind": "Unit"})),
        OpenAdjacent::Unit
    ));
    assert!(serde_json::from_value::<OpenAdjacent>(
        json!({"kind": "Value", "data": {"message": 2}})
    )
    .is_err());
    assert!(serde_json::from_value::<OpenAdjacent>(json!({"data": {"message": "ok"}})).is_err());
    assert!(serde_json::from_value::<OpenAdjacent>(json!({"kind": null})).is_err());
}

#[test]
fn external_requires_valid_envelope() {
    assert!(
        matches!(decode::<OpenExternal>(json!("Future")), OpenExternal::Unknown { tag, raw } if tag == "Future" && raw == json!("Future"))
    );
    let raw = json!({"Future": {"nested": [1, 2]}});
    assert!(
        matches!(decode::<OpenExternal>(raw.clone()), OpenExternal::Unknown { tag, raw: retained } if tag == "Future" && retained == raw)
    );
    assert!(matches!(
        decode::<OpenExternal>(json!("Unit")),
        OpenExternal::Unit
    ));
    assert!(matches!(
        decode::<OpenExternal>(json!({"Number": 7})),
        OpenExternal::Number(7)
    ));
    for raw in [
        json!({}),
        json!({"One": 1, "Two": 2}),
        json!(42),
        json!(null),
        json!({"Value": {"message": 2}}),
        json!("Value"),
    ] {
        assert!(serde_json::from_value::<OpenExternal>(raw).is_err());
    }
}

#[test]
fn source_unknown_name_does_not_collide() {
    assert!(matches!(
        decode::<OpenUnit>(json!("Unknown")),
        OpenUnit::Unknown
    ));
    assert!(
        matches!(decode::<OpenUnit>(json!("Future")), OpenUnit::Unknown_ { tag, raw } if tag == "Future" && raw == json!("Future"))
    );
}

#[test]
fn generic_enum_retains_generic_type() {
    let known: OpenGeneric<String> = decode(json!({"kind": "Value", "value": "hello"}));
    assert!(matches!(&known, OpenGeneric::Value { value } if value == "hello"));
    assert_eq!(
        serde_json::to_value(&known).unwrap(),
        json!({"kind": "Value", "value": "hello"})
    );
    let raw = json!({"kind": "Future", "value": [1]});
    let unknown: OpenGeneric<String> = decode(raw.clone());
    assert!(
        matches!(&unknown, OpenGeneric::Unknown { tag, raw: retained } if tag == "Future" && retained == &raw)
    );
    assert_eq!(serde_json::to_value(&unknown).unwrap(), raw);
    assert!(
        serde_json::from_value::<OpenGeneric<String>>(json!({"kind": "Value", "value": 42}))
            .is_err()
    );
}

#[test]
fn two_type_parameters_do_not_shadow_serde_method_parameters() {
    let known: OpenGenericDS<String, u32> =
        decode(json!({"kind": "Value", "first": "hello", "second": 7}));
    assert!(
        matches!(&known, OpenGenericDS::Value { first, second } if first == "hello" && *second == 7)
    );
    assert_eq!(
        serde_json::to_value(&known).unwrap(),
        json!({"kind": "Value", "first": "hello", "second": 7})
    );

    let raw = json!({"kind": "Future", "first": [1], "second": {"v": 2}});
    let unknown: OpenGenericDS<String, u32> = decode(raw.clone());
    assert!(
        matches!(&unknown, OpenGenericDS::Unknown { tag, raw: retained } if tag == "Future" && retained == &raw)
    );
    assert_eq!(serde_json::to_value(&unknown).unwrap(), raw);
    assert!(serde_json::from_value::<OpenGenericDS<String, u32>>(
        json!({"kind": "Value", "first": 3, "second": 7})
    )
    .is_err());
}

#[test]
fn flattened_and_untagged_containers_preserve_open_values() {
    let value: FlattenEnvelope = decode(json!({
        "ordinary": "FutureOrdinary", "first": "FutureFirst",
        "second": {"kind": "FutureSecond", "extra": true}
    }));
    assert!(matches!(value.ordinary, OpenUnit::Unknown_ { tag, .. } if tag == "FutureOrdinary"));
    assert!(
        matches!(value.first_part.first, OpenUnit::Unknown_ { tag, .. } if tag == "FutureFirst")
    );
    assert!(
        matches!(value.second_part.second, OpenInternal::Unknown { tag, .. } if tag == "FutureSecond")
    );
    let value: UntaggedWrapper = decode(json!({"kind": "Future", "extra": true}));
    assert!(
        matches!(value, UntaggedWrapper::Tagged(OpenInternal::Unknown { tag, .. }) if tag == "Future")
    );
    let value: OpenNested = decode(json!({"kind": "Wrapped", "value": {"kind": "Future"}}));
    assert!(
        matches!(value, OpenNested::Wrapped { value: UntaggedWrapper::Tagged(OpenInternal::Unknown { tag, .. }) } if tag == "Future")
    );
    assert!(serde_json::from_value::<OpenNested>(json!({
        "kind": "Wrapped", "value": {"kind": "Dog", "breed": 4}
    }))
    .is_err());
    assert!(serde_json::from_value::<OpenNested>(json!({
        "kind": "Wrapped", "value": 4
    }))
    .is_err());
}

#[test]
fn directly_flattened_open_enum_preserves_parent_fields() {
    let value: FlattenEnumEnvelope = decode(json!({"id": 7, "kind": "Future", "extra": true}));
    assert_eq!(value.id, 7);
    assert!(matches!(value.value, OpenInternal::Unknown { tag, raw }
        if tag == "Future" && raw == json!({"kind": "Future", "extra": true})));
    let value: FlattenEnumEnvelope = decode(json!({"id": 8, "kind": "Dog", "breed": "Husky"}));
    assert_eq!(value.id, 8);
    assert!(matches!(value.value, OpenInternal::Dog { breed } if breed == "Husky"));
    assert!(serde_json::from_value::<FlattenEnumEnvelope>(json!({
        "id": 9, "kind": "Dog", "breed": 3
    }))
    .is_err());
}

#[test]
fn optional_flattened_open_enum_preserves_absence_and_unknown() {
    let value: OptionalFlattenEnumEnvelope = decode(json!({"id": 7}));
    assert_eq!(value.id, 7);
    assert!(value.value.is_none());
    let value: OptionalFlattenEnumEnvelope =
        decode(json!({"id": 8, "kind": "Future", "extra": true}));
    assert_eq!(value.id, 8);
    assert!(
        matches!(value.value, Some(OpenInternal::Unknown { tag, raw })
        if tag == "Future" && raw == json!({"kind": "Future", "extra": true}))
    );
    let value: OptionalFlattenEnumEnvelope =
        decode(json!({"id": 9, "kind": "Dog", "breed": "Husky"}));
    assert!(matches!(value.value, Some(OpenInternal::Dog { breed }) if breed == "Husky"));
    // Serde's optional flatten adapter converts a failed field decode to None;
    // the malformed case is never classified as an unknown enum variant.
    let malformed: OptionalFlattenEnumEnvelope =
        decode(json!({"id": 10, "kind": "Dog", "breed": 3}));
    assert!(malformed.value.is_none());
}

#[test]
fn ordinary_other_and_explicit_open_have_distinct_behavior() {
    assert!(matches!(
        decode::<FallbackInternal>(json!({"kind": "Future", "v": 1})),
        FallbackInternal::Other
    ));
    assert!(matches!(
        decode::<FallbackAdjacent>(json!({"kind": "Future"})),
        FallbackAdjacent::Other
    ));
    assert!(matches!(
        decode::<FallbackAdjacent>(json!({"kind": "Future", "data": null})),
        FallbackAdjacent::Other
    ));
    assert!(serde_json::from_value::<FallbackAdjacent>(
        json!({"kind": "Future", "data": {"v": 1}})
    )
    .is_err());
    let raw = json!({"kind": "Future", "v": 1});
    assert!(
        matches!(decode::<OpenWithFallback>(raw.clone()), OpenWithFallback::Unknown { tag, raw: retained } if tag == "Future" && retained == raw)
    );
    assert!(matches!(
        decode::<OpenWithFallback>(json!({"kind": "Other"})),
        OpenWithFallback::Other
    ));
}

#[test]
fn unknown_error_serializes_original_wire_value() {
    let raw = json!({"kind": "Future", "detail": {"reason": "changed"}});
    let error: OpenInternal = decode(raw.clone());
    let _: &dyn std::error::Error = &error;
    assert_eq!(serde_json::to_value(&error).unwrap(), raw);
    assert_eq!(error.to_string(), raw.to_string());
    assert_eq!(
        serde_json::to_value(OpenInternal::Dog {
            breed: "Collie".into()
        })
        .unwrap(),
        json!({"kind": "Dog", "breed": "Collie"})
    );
}

#[test]
fn generated_source_passes_optional_stub_typecheck() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../target/enum-compatibility/schema.json"
    );
    let mut schema: reflectapi::Schema =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    // The stub typechecker predates streaming support; the Cargo tests above
    // compile the exported streaming client against the real runtime.
    schema
        .functions
        .retain(|function| function.name != "enums.stream");
    let mut config = reflectapi::codegen::rust::Config::default();
    config.typecheck(true);
    reflectapi::codegen::rust::generate(schema, &config).unwrap();
}
