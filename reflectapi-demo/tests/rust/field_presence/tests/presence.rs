use reflectapi_field_presence_rust::types::field_presence::fixtures::{input, output, Profile, Shared, Tuple};
use serde_json::json;

#[test]
fn generated_requests_keep_omitted_and_explicit_null_fields_distinct() {
    let omitted: input::Fields = serde_json::from_value(json!({"custom": null})).unwrap();
    assert_eq!(
        serde_json::to_value(omitted).unwrap(),
        json!({"custom": null})
    );
    let wire = json!({
        "plain": null, "custom": null, "defaulted": null, "serializer_only": null,
        "skipped": null, "three_state": null, "default_three_state": null,
    });
    let explicit: input::Fields = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(explicit).unwrap(), wire);
}

#[test]
fn generated_responses_keep_required_nullable_fields_on_the_wire() {
    let wire = json!({
        "plain": null, "custom": null, "defaulted": null, "serializer_only": null,
        "three_state": null, "default_three_state": null,
    });
    let response: output::Fields = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(response).unwrap(), wire);
    assert!(serde_json::from_value::<Tuple>(json!([])).is_err());
    assert!(serde_json::from_value::<Tuple>(json!([null, null])).is_ok());
}

#[test]
fn generated_struct_variants_and_nested_generic_requests_keep_null() {
    let variant = json!({"kind": "Value", "value": null});
    let request: input::Variant = serde_json::from_value(variant.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), variant);
    let omitted: input::Variant = serde_json::from_value(json!({"kind": "Value"})).unwrap();
    assert_eq!(
        serde_json::to_value(omitted).unwrap(),
        json!({"kind": "Value"})
    );
    for child in [
        json!({}),
        json!({"value": null}),
        json!({"value": "present"}),
    ] {
        let wire = json!({"one": child, "many": [child]});
        let nested: input::Nested = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(nested).unwrap(), wire);
    }
}

#[test]
fn defaulted_fields_of_generated_types_may_be_omitted() {
    // The generated `Memo` doesn't implement `Default`, so its fields are
    // `Option`s: `None` leaves the key out for the server to default.
    let wire = json!({"memo": {"text": "a"}, "boxed": {"text": "b"}});
    let defaulted: input::Defaulted = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(defaulted).unwrap(), wire);
    let empty: input::Defaulted = serde_json::from_value(json!({})).unwrap();
    assert!(empty.memo.is_none() && empty.boxed.is_none());
    assert_eq!(serde_json::to_value(empty).unwrap(), json!({}));
}

#[test]
#[allow(deprecated)]
fn generated_responses_may_omit_deprecated_fields() {
    let profile: Profile = serde_json::from_value(json!({"id": 1})).unwrap();
    assert_eq!(profile.legacy_tags, None);
    assert_eq!(profile.legacy_code, "");
    assert!(profile.legacy_config.is_none());
    assert!(serde_json::from_value::<Shared>(json!({"id": 1})).is_err());
}
