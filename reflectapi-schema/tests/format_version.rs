//! The schema JSON records its format version, so a reader refuses a schema
//! newer than it understands instead of generating a client that's silently
//! wrong (#246).

use reflectapi_schema::{Schema, SchemaFormat, SCHEMA_FORMAT_VERSION};

fn parse(json: serde_json::Value) -> Result<Schema, String> {
    serde_json::from_str(&json.to_string()).map_err(|error| error.to_string())
}

#[test]
fn a_schema_is_written_with_its_format_version_first() {
    let json = serde_json::to_string(&Schema::new()).unwrap();
    assert!(
        json.starts_with(&format!("{{\"format_version\":{SCHEMA_FORMAT_VERSION},")),
        "{json}"
    );
}

#[test]
fn a_schema_without_a_format_version_is_format_1() {
    let schema = parse(serde_json::json!({"name": "api"})).unwrap();
    assert_eq!(schema.format_version, 1);
}

/// A schema written before versions were recorded, by the release before
/// this one: the checked-in demo schema.
#[test]
fn a_schema_from_before_versions_were_recorded_is_read() {
    let json = include_str!("fixtures/unversioned-0.25.json");
    assert!(!json.contains("format_version"));
    let schema: Schema = serde_json::from_str(json).unwrap();
    assert_eq!(schema.format_version, 1);
    assert!(schema.functions().count() > 0);
    assert!(schema.input_types().types().count() > 0);
}

#[test]
fn every_format_up_to_the_current_one_is_read() {
    for version in 1..=SCHEMA_FORMAT_VERSION {
        let schema = parse(serde_json::json!({"format_version": version, "name": "api"})).unwrap();
        assert_eq!(schema.format_version, version);
    }
}

/// A newer format is refused by its version, before the reader reaches a part
/// it can't parse: here a route kind it doesn't know.
#[test]
fn a_newer_format_is_refused_before_the_rest_is_read() {
    let newer = SCHEMA_FORMAT_VERSION + 1;
    let error = parse(serde_json::json!({
        "format_version": newer,
        "name": "api",
        "functions": [{
            "name": "events.subscribe",
            "path": "",
            "output_kind": "a_route_kind_from_the_future",
            "serialization": ["json"],
            "readonly": false
        }]
    }))
    .unwrap_err();
    assert!(
        error.contains(&format!(
            "this schema is in format version {newer}, but reflectapi-schema"
        )),
        "{error}"
    );
    assert!(error.contains("upgrade reflectapi-cli"), "{error}");
    assert!(!error.contains("unknown variant"), "{error}");
}

/// Within a format version, changes are additive: an older reader ignores
/// fields it doesn't know, at every level.
#[test]
fn unknown_fields_are_ignored_within_a_format() {
    let schema = parse(serde_json::json!({
        "format_version": SCHEMA_FORMAT_VERSION,
        "name": "api",
        "a_new_schema_field": true,
        "functions": [{
            "name": "pets.get",
            "path": "",
            "output_kind": "complete",
            "serialization": ["json"],
            "readonly": true,
            "a_new_function_field": 1
        }],
        "output_types": {"types": [{
            "kind": "struct",
            "name": "Pet",
            "fields": {"named": [{
                "name": "name",
                "type": {"name": "std::string::String"},
                "required": true,
                "a_new_field_field": "x"
            }]},
            "a_new_struct_field": []
        }]}
    }))
    .unwrap();
    assert_eq!(schema.functions().count(), 1);
    assert!(schema.output_types().get_type("Pet").is_some());
}

#[test]
fn merging_schemas_keeps_the_newer_format() {
    let mut older = parse(serde_json::json!({"name": "api"})).unwrap();
    older.extend(Schema::new());
    assert_eq!(older.format_version, SCHEMA_FORMAT_VERSION);
}

/// JSON key order isn't significant, and a tool may reorder keys. `Schema`
/// alone checks the version only when it comes first; reading the JSON into
/// `SchemaFormat` first checks it wherever it is.
#[test]
fn a_newer_format_is_refused_wherever_its_key_is() {
    let newer = SCHEMA_FORMAT_VERSION + 1;
    let json = format!(
        r#"{{
            "name": "api",
            "functions": [{{
                "name": "events.subscribe",
                "path": "",
                "output_kind": "a_route_kind_from_the_future",
                "serialization": ["json"],
                "readonly": false
            }}],
            "format_version": {newer}
        }}"#
    );
    let error = serde_json::from_str::<SchemaFormat>(&json)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(&format!("this schema is in format version {newer}")),
        "{error}"
    );
    // Without the probe, the unknown route kind is reported first.
    let error = serde_json::from_str::<Schema>(&json)
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown variant"), "{error}");
}

#[test]
fn the_format_probe_reads_a_current_or_unversioned_schema() {
    let probe: SchemaFormat =
        serde_json::from_str(include_str!("fixtures/unversioned-0.25.json")).unwrap();
    assert_eq!(probe.format_version, 1);
    let json = serde_json::to_string(&Schema::new()).unwrap();
    let probe: SchemaFormat = serde_json::from_str(&json).unwrap();
    assert_eq!(probe.format_version, SCHEMA_FORMAT_VERSION);
}
