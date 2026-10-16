#![allow(dead_code)]

use reflectapi::{Input, Output};
use serde::{Deserialize, Serialize};

type Maybe<T> = Option<T>;

fn deserialize_value<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer)
}

fn serialize_value<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: Serialize,
{
    value.serialize(serializer)
}

#[derive(Input, Output, Serialize, Deserialize)]
struct Presence {
    plain: Option<u8>,
    alias: Maybe<u8>,
    #[serde(default)]
    defaulted: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<u8>,
    #[serde(serialize_with = "serialize_value")]
    serializer: Option<u8>,
    #[serde(deserialize_with = "deserialize_value")]
    deserializer: Option<u8>,
    #[serde(default, deserialize_with = "deserialize_value")]
    defaulted_deserializer: Option<u8>,
}

fn reflected_input<T: Input>() -> reflectapi::Type {
    let mut types = reflectapi::Typespace::new();
    let reference = T::reflectapi_input_type(&mut types);
    types.get_type(reference.name()).unwrap().clone()
}

fn reflected_output<T: Output>() -> reflectapi::Type {
    let mut types = reflectapi::Typespace::new();
    let reference = T::reflectapi_output_type(&mut types);
    types.get_type(reference.name()).unwrap().clone()
}

fn requirements(ty: reflectapi::Type) -> Vec<(String, bool)> {
    ty.as_struct()
        .unwrap()
        .fields
        .iter()
        .map(|field| (field.name.clone(), field.required))
        .collect()
}

#[test]
fn input_presence_is_normalized_without_changing_output_presence() {
    assert_eq!(
        requirements(reflected_input::<Presence>()),
        [
            ("plain", false),
            ("alias", false),
            ("defaulted", false),
            ("skipped", false),
            ("serializer", false),
            ("deserializer", true),
            ("defaulted_deserializer", false),
        ]
        .map(|(name, required)| (name.to_owned(), required))
    );
    assert_eq!(
        requirements(reflected_output::<Presence>()),
        [
            ("plain", true),
            ("alias", true),
            ("defaulted", true),
            ("skipped", false),
            ("serializer", true),
            ("deserializer", true),
            ("defaulted_deserializer", true),
        ]
        .map(|(name, required)| (name.to_owned(), required))
    );

    let decoded: Presence = serde_json::from_str(r#"{"deserializer":null}"#).unwrap();
    assert!(decoded.plain.is_none());
    assert!(serde_json::from_str::<Presence>("{}").is_err());
    let output = serde_json::to_value(decoded).unwrap();
    assert!(output.get("plain").unwrap().is_null());
    assert!(output.get("skipped").is_none());
}

#[derive(Input, Output, Deserialize)]
struct Generic<T> {
    direct: Option<T>,
    unresolved: T,
}

#[test]
fn option_wrappers_are_normalized_after_restoring_generic_parameters() {
    let ty = reflected_input::<Generic<Option<u8>>>();
    let fields = &ty.as_struct().unwrap().fields;
    assert!(!fields[0].required);
    assert_eq!(fields[0].type_ref.name(), "std::option::Option");
    assert_eq!(fields[0].type_ref.arguments[0].name(), "T");
    // Presence of an unresolved generic is deliberately not inferred from
    // whichever concrete instantiation first registers this definition.
    assert!(fields[1].required);
    assert_eq!(fields[1].type_ref.name(), "T");
    assert!(requirements(reflected_output::<Generic<u8>>())[0].1);
}

#[derive(Input, Output)]
struct Transformed {
    #[reflectapi(transform = "reflectapi::transforms::make_required")]
    value: reflectapi::Option<u8>,
}

#[test]
fn user_transform_runs_after_input_presence_normalization() {
    let ty = reflected_input::<Transformed>();
    let field = &ty.as_struct().unwrap().fields[0];
    assert!(field.required);
    assert_eq!(field.type_ref.name(), "std::option::Option");
}

#[derive(Input, Output, Deserialize)]
enum Variant {
    Named { value: Option<u8> },
}

#[test]
fn enum_named_fields_use_the_same_directional_rules() {
    let input = reflected_input::<Variant>();
    let output = reflected_output::<Variant>();
    assert!(
        !input.as_enum().unwrap().variants[0]
            .fields
            .iter()
            .next()
            .unwrap()
            .required
    );
    assert!(
        output.as_enum().unwrap().variants[0]
            .fields
            .iter()
            .next()
            .unwrap()
            .required
    );
    assert!(serde_json::from_str::<Variant>(r#"{"Named":{}}"#).is_ok());
}

#[derive(Input, Deserialize)]
struct OnePosition(Option<u8>);

#[derive(Input, Deserialize)]
struct TwoPositions(Option<u8>, Option<u8>);

#[derive(Input, Deserialize)]
enum TupleVariant {
    One(Option<u8>),
    Two(Option<u8>, Option<u8>),
}

#[test]
fn positional_option_fields_do_not_infer_missing_key_acceptance() {
    assert!(requirements(reflected_input::<OnePosition>())[0].1);
    assert!(requirements(reflected_input::<TwoPositions>())
        .iter()
        .all(|(_, required)| *required));
    let ty = reflected_input::<TupleVariant>();
    assert!(ty
        .as_enum()
        .unwrap()
        .variants
        .iter()
        .all(|variant| variant.fields.iter().all(|field| field.required)));
    assert!(serde_json::from_str::<OnePosition>("[]").is_err());
    assert!(serde_json::from_str::<TwoPositions>("[]").is_err());
    assert!(serde_json::from_str::<TwoPositions>("[null]").is_err());
    assert!(serde_json::from_str::<TupleVariant>(r#"{"One":[]}"#).is_err());
    assert!(serde_json::from_str::<TupleVariant>(r#"{"Two":[null]}"#).is_err());
}

#[derive(Input, Deserialize)]
struct ThreeState {
    plain: reflectapi::Option<u8>,
    #[serde(default)]
    defaulted: reflectapi::Option<u8>,
}

#[test]
fn missing_reflectapi_option_uses_none_unless_defaulted() {
    let value: ThreeState = serde_json::from_str("{}").unwrap();
    assert!(matches!(value.plain, reflectapi::Option::None));
    assert!(matches!(value.defaulted, reflectapi::Option::Undefined));
    assert!(requirements(reflected_input::<ThreeState>())
        .iter()
        .all(|(_, required)| !required));
}

#[derive(Deserialize)]
struct GenericValue<T> {
    value: T,
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct GenericCustomValue<T> {
    #[serde(deserialize_with = "deserialize_value")]
    value: T,
}

#[test]
fn serde_map_missing_option_behavior_depends_on_custom_deserializer() {
    type Error = serde::de::value::Error;
    let empty =
        || serde::de::value::MapDeserializer::<_, Error>::new(std::iter::empty::<(&str, u8)>());
    assert!(GenericValue::<Option<u8>>::deserialize(empty())
        .unwrap()
        .value
        .is_none());
    assert!(GenericCustomValue::<Option<u8>>::deserialize(empty()).is_err());
}

#[test]
fn manual_and_legacy_json_required_nullable_contract_is_preserved() {
    let field: reflectapi::Field = serde_json::from_str(
        r#"{"name":"value","type":{"name":"std::option::Option","arguments":[{"name":"u8"}]},"required":true}"#,
    ).unwrap();
    let mut ty = reflectapi::Struct::new("Manual");
    ty.fields = reflectapi::Fields::Named(vec![field]);
    let mut ty = reflectapi::Type::from(ty);
    ty.__internal_rebind_generic_parameters(&Default::default(), &reflectapi::Typespace::new());
    assert!(ty.as_struct().unwrap().fields[0].required);
}

#[derive(Input, Output, Deserialize)]
struct WrappedOption {
    #[reflectapi(transform = "reflectapi::transforms::fallback_recursively")]
    value: std::sync::Arc<Option<u8>>,
}

#[test]
fn transparent_option_presence_is_resolved_before_fallback_transforms() {
    let input = reflected_input::<WrappedOption>();
    let field = &input.as_struct().unwrap().fields[0];
    assert_eq!(field.type_ref.name(), "std::option::Option");
    assert!(!field.required);
    assert!(
        reflected_output::<WrappedOption>()
            .as_struct()
            .unwrap()
            .fields[0]
            .required
    );
    assert!(serde_json::from_str::<WrappedOption>("{}")
        .unwrap()
        .value
        .is_none());
}
