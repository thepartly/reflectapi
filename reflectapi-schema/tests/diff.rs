use reflectapi_schema::diff::{diff, Severity};
use reflectapi_schema::Schema;
use serde_json::{json, Value};

fn primitives() -> Vec<Value> {
    ["std::string::String", "u32", "u64", "bool"]
        .iter()
        .map(|name| json!({ "kind": "primitive", "name": name }))
        .chain([json!({ "kind": "primitive", "name": "std::vec::Vec", "parameters": [{ "name": "T" }] })])
        .chain([json!({ "kind": "primitive", "name": "std::option::Option", "parameters": [{ "name": "T" }] })])
        .collect()
}

fn with_primitives(types: Vec<Value>) -> Value {
    Value::Array(primitives().into_iter().chain(types).collect())
}

fn field(name: &str, ty: Value, required: bool) -> Value {
    json!({ "name": name, "type": ty, "required": required })
}

fn named(name: &str, fields: Vec<Value>) -> Value {
    json!({ "kind": "struct", "name": name, "fields": { "named": fields } })
}

fn reference(name: &str) -> Value {
    json!({ "name": name })
}

fn optional(inner: Value) -> Value {
    json!({ "name": "std::option::Option", "arguments": [inner] })
}

fn breaking(old: &Schema, new: &Schema) -> Vec<String> {
    diff(old, new)
        .into_iter()
        .filter(|c| c.severity == Severity::Breaking)
        .map(|c| c.to_string())
        .collect()
}

#[test]
fn removed_function_carries_its_deprecation() {
    let old: Schema = serde_json::from_value(json!({
        "name": "test",
        "functions": [{ "name": "f", "path": "/api", "output_kind": "complete", "deprecation_note": "use g", "sunset": "2026-01-01" }],
    }))
    .unwrap();
    let new: Schema = serde_json::from_value(json!({ "name": "test" })).unwrap();
    assert_eq!(
        breaking(&old, &new),
        vec!["f: function removed (deprecated, sunset 2026-01-01)"]
    );
}

fn schema_with(functions: Vec<Value>, types: Vec<Value>) -> Schema {
    let types = with_primitives(types);
    let functions: Vec<Value> = functions
        .into_iter()
        .map(|mut f| {
            f["output_kind"] = f.get("output_kind").cloned().unwrap_or(json!("complete"));
            f
        })
        .collect();
    serde_json::from_value(json!({
        "name": "test",
        "functions": functions,
        "input_types": { "types": types },
        "output_types": { "types": types },
    }))
    .unwrap()
}

fn function(fields: Value) -> Value {
    let mut f = json!({ "name": "f", "path": "/api" });
    f.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    f
}

fn all(old: &Schema, new: &Schema) -> Vec<String> {
    diff(old, new)
        .into_iter()
        .map(|c| format!("{:?} {c}", c.severity))
        .collect()
}

fn both_sides(types_old: Vec<Value>, types_new: Vec<Value>) -> (Schema, Schema) {
    let f = function(json!({ "input_type": reference("T"), "output_type": reference("T") }));
    (
        schema_with(vec![f.clone()], types_old),
        schema_with(vec![f], types_new),
    )
}

fn single_field(ty: Value) -> Value {
    named("T", vec![field("a", ty, true)])
}

#[test]
fn added_functions_are_compatible() {
    let old = schema_with(vec![], vec![]);
    let new = schema_with(vec![function(json!({}))], vec![]);
    assert_eq!(all(&old, &new), vec!["Compatible f: function added"]);
}

#[test]
fn route_changes_break() {
    let old = schema_with(
        vec![function(
            json!({ "readonly": true, "serialization": ["json", "msgpack"] }),
        )],
        vec![],
    );
    let new = schema_with(
        vec![function(
            json!({ "path": "/api/v2", "serialization": ["json"] }),
        )],
        vec![],
    );
    assert_eq!(
        all(&old, &new),
        vec![
            "Breaking f: path changed from `/api` to `/api/v2`",
            "Breaking f: no longer readonly, so no longer accepts GET",
            "Breaking f: no longer supports Msgpack serialization",
        ],
    );
}

#[test]
fn unspecified_serialization_means_json() {
    let old = schema_with(vec![function(json!({}))], vec![]);
    let new = schema_with(vec![function(json!({ "serialization": ["json"] }))], vec![]);
    assert_eq!(all(&old, &new), Vec::<String>::new());
}

#[test]
fn switching_between_stream_and_single_response_breaks() {
    let old = schema_with(
        vec![function(json!({ "output_type": reference("u32") }))],
        vec![],
    );
    let new = schema_with(
        vec![function(
            json!({ "output_kind": "stream", "item_type": reference("u32") }),
        )],
        vec![],
    );
    assert_eq!(
        all(&old, &new),
        vec!["Breaking f output: changed from a single response to a stream"],
    );
}

#[test]
fn stream_items_are_compared_as_output() {
    let stream =
        |item: &str| function(json!({ "output_kind": "stream", "item_type": reference(item) }));
    let old = schema_with(vec![stream("u32")], vec![]);
    let new = schema_with(vec![stream("u64")], vec![]);
    assert_eq!(
        all(&old, &new),
        vec!["Breaking f output stream item: changed from `u32` to `u64`"],
    );
}

#[test]
fn bodies_added_and_removed() {
    let optional_body = named("Opt", vec![field("a", optional(reference("u32")), false)]);
    let required_body = named("Req", vec![field("a", reference("u32"), true)]);
    let types = vec![optional_body, required_body];

    let empty = schema_with(vec![function(json!({}))], types.clone());
    let optional_input = schema_with(
        vec![function(json!({ "input_type": reference("Opt") }))],
        types.clone(),
    );
    let required_input = schema_with(
        vec![function(json!({ "input_type": reference("Req") }))],
        types.clone(),
    );
    let with_output = schema_with(
        vec![function(json!({ "output_type": reference("Req") }))],
        types,
    );

    assert_eq!(
        all(&empty, &optional_input),
        vec!["Compatible f input body: now read, all of it optional"],
    );
    assert_eq!(
        all(&empty, &required_input),
        vec!["Breaking f input body: now required: a"],
    );
    assert_eq!(
        all(&required_input, &empty),
        vec!["Compatible f input body: no longer exchanged"],
    );
    assert_eq!(
        all(&empty, &with_output),
        vec!["Compatible f output body: now returned"],
    );
    assert_eq!(
        all(&with_output, &empty),
        vec!["Breaking f output body: no longer exchanged"],
    );
}

#[test]
fn headers_follow_the_same_rules_as_bodies() {
    let headers = |fields: Vec<Value>| named("H", fields);
    let f =
        function(json!({ "input_headers": reference("H"), "response_headers": reference("H") }));
    let old = schema_with(
        vec![f.clone()],
        vec![headers(vec![field(
            "x-trace",
            reference("std::string::String"),
            true,
        )])],
    );
    let new = schema_with(
        vec![f],
        vec![headers(vec![field(
            "x-tenant",
            reference("std::string::String"),
            true,
        )])],
    );
    assert_eq!(
        all(&old, &new),
        vec![
            "Breaking f input headers.x-tenant: required field added",
            "Breaking f output headers.x-trace: required field removed",
            "Compatible f input headers.x-trace: field removed, the server ignores it if sent",
            "Compatible f output headers.x-tenant: field added",
        ],
    );
}

#[test]
fn becoming_non_nullable_breaks_input_only() {
    let (old, new) = both_sides(
        vec![single_field(optional(reference("u32")))],
        vec![single_field(reference("u32"))],
    );
    assert_eq!(
        all(&old, &new),
        vec![
            "Breaking f input body.a: now required",
            "Breaking f input body.a: no longer nullable",
            "Compatible f output body.a: no longer nullable",
        ],
    );
}

#[test]
fn unrelated_types_break_both_ways() {
    let (old, new) = both_sides(
        vec![single_field(reference("std::string::String"))],
        vec![single_field(reference("u32"))],
    );
    assert_eq!(
        all(&old, &new),
        vec![
            "Breaking f input body.a: changed from `std::string::String` to `u32`",
            "Breaking f output body.a: changed from `std::string::String` to `u32`",
        ],
    );

    let (old, new) = both_sides(
        vec![named("T", vec![])],
        vec![json!({ "kind": "enum", "name": "T", "variants": [] })],
    );
    assert_eq!(
        all(&old, &new),
        vec![
            "Breaking f input body: changed from struct `T` to enum `T`",
            "Breaking f output body: changed from struct `T` to enum `T`",
        ],
    );
}

#[test]
fn numbers_widen_for_input_and_narrow_for_output() {
    let case = |from: &str, to: &str| {
        let (old, new) = both_sides(
            vec![single_field(reference(from))],
            vec![single_field(reference(to))],
        );
        breaking(&old, &new)
    };
    assert_eq!(
        case("u32", "i64"),
        vec!["f output body.a: changed from `u32` to `i64`"]
    );
    assert_eq!(
        case("i64", "u32"),
        vec!["f input body.a: changed from `i64` to `u32`"]
    );
    assert_eq!(
        case("u64", "i64"),
        vec![
            "f input body.a: changed from `u64` to `i64`",
            "f output body.a: changed from `u64` to `i64`",
        ],
    );
    assert_eq!(
        case("f32", "f64"),
        vec!["f output body.a: changed from `f32` to `f64`"]
    );
    assert_eq!(
        case("u32", "f64"),
        vec!["f output body.a: changed from `u32` to `f64`"]
    );
}

#[test]
fn falling_back_to_a_plain_string_loosens_the_format() {
    let uuid = json!({
        "kind": "primitive",
        "name": "uuid::Uuid",
        "format": "uuid",
        "fallback": reference("std::string::String"),
    });
    let (old, new) = both_sides(
        vec![uuid.clone(), single_field(reference("uuid::Uuid"))],
        vec![uuid.clone(), single_field(reference("std::string::String"))],
    );
    assert_eq!(
        breaking(&old, &new),
        vec!["f output body.a: changed from `uuid::Uuid` to `std::string::String`, no longer only uuid"],
    );

    let (old, new) = both_sides(
        vec![uuid.clone(), single_field(reference("std::string::String"))],
        vec![uuid, single_field(reference("uuid::Uuid"))],
    );
    assert_eq!(
        breaking(&old, &new),
        vec!["f input body.a: changed from `std::string::String` to `uuid::Uuid`, now only uuid"],
    );
}

#[test]
fn unwrapping_a_box_changes_nothing_on_the_wire() {
    let boxed = json!({
        "kind": "primitive",
        "name": "std::boxed::Box",
        "parameters": [{ "name": "T" }],
        "fallback": reference("T"),
    });
    let (old, new) = both_sides(
        vec![
            boxed.clone(),
            single_field(json!({ "name": "std::boxed::Box", "arguments": [reference("u32")] })),
        ],
        vec![boxed, single_field(reference("u32"))],
    );
    assert_eq!(breaking(&old, &new), Vec::<String>::new());
}

#[test]
fn a_changed_format_breaks_but_a_newly_declared_one_does_not() {
    let id = |format: Option<&str>| {
        let mut p = json!({ "kind": "primitive", "name": "Id" });
        if let Some(format) = format {
            p["format"] = json!(format);
        }
        p
    };
    let case = |old: Option<&str>, new: Option<&str>| {
        let (o, n) = both_sides(
            vec![id(old), single_field(reference("Id"))],
            vec![id(new), single_field(reference("Id"))],
        );
        breaking(&o, &n)
    };
    assert_eq!(case(None, Some("uuid")), Vec::<String>::new());
    assert_eq!(
        case(Some("uuid"), Some("ulid")),
        vec![
            "f input body.a: format changed from `uuid` to `ulid`",
            "f output body.a: format changed from `uuid` to `ulid`",
        ],
    );
}

#[test]
fn enum_tagging_and_discriminants_are_part_of_the_contract() {
    let tagged = |representation: Value| {
        json!({
            "kind": "enum",
            "name": "T",
            "representation": representation,
            "variants": [{ "name": "a", "fields": "none" }],
        })
    };
    let (old, new) = both_sides(
        vec![tagged(json!({ "internal": { "tag": "kind" } }))],
        vec![tagged(json!({ "internal": { "tag": "type" } }))],
    );
    assert_eq!(
        breaking(&old, &new),
        vec![
            "f input body: changed from internally tagged by `kind` to internally tagged by `type`",
            "f output body: changed from internally tagged by `kind` to internally tagged by `type`",
        ],
    );

    let numbered = |discriminant: i64| {
        json!({
            "kind": "enum",
            "name": "T",
            "representation": "none",
            "variants": [{ "name": "a", "fields": "none", "discriminant": discriminant }],
        })
    };
    let (old, new) = both_sides(vec![numbered(1)], vec![numbered(2)]);
    assert_eq!(
        breaking(&old, &new),
        vec![
            "f input body::a: discriminant changed",
            "f output body::a: discriminant changed"
        ],
    );
}

#[test]
fn variants_mirror_fields() {
    let enumeration =
        |variants: Vec<Value>| json!({ "kind": "enum", "name": "T", "variants": variants });
    let unit = |name: &str| json!({ "name": name, "fields": "none" });
    let catch_all = json!({ "name": "unknown", "fields": "none", "other": true });

    let (old, new) = both_sides(
        vec![enumeration(vec![unit("a"), unit("b")])],
        vec![enumeration(vec![unit("a"), unit("c")])],
    );
    assert_eq!(
        all(&old, &new),
        vec![
            "Breaking f input body::b: variant removed",
            "Breaking f output body::c: variant added, old clients can't read it",
            "Compatible f input body::c: variant added",
            "Compatible f output body::b: variant removed",
        ],
    );

    let (old, new) = both_sides(
        vec![enumeration(vec![unit("a"), catch_all.clone()])],
        vec![enumeration(vec![unit("a"), unit("b"), catch_all])],
    );
    assert_eq!(breaking(&old, &new), Vec::<String>::new());

    let (old, new) = both_sides(
        vec![enumeration(vec![unit("a")])],
        vec![enumeration(vec![
            json!({ "name": "a", "fields": { "named": [field("x", reference("u32"), true)] } }),
        ])],
    );
    assert_eq!(
        breaking(&old, &new),
        vec![
            "f input body::a: changed from no fields to named fields",
            "f output body::a: changed from no fields to named fields",
        ],
    );
}

#[test]
fn tuples_compare_by_position() {
    let tuple = |types: Vec<&str>| {
        let fields: Vec<Value> = types
            .iter()
            .enumerate()
            .map(|(i, t)| field(&i.to_string(), reference(t), true))
            .collect();
        json!({ "kind": "struct", "name": "T", "fields": { "unnamed": fields } })
    };
    let (old, new) = both_sides(vec![tuple(vec!["u32", "u32"])], vec![tuple(vec!["u32"])]);
    assert_eq!(
        breaking(&old, &new),
        vec![
            "f input body: tuple of 2 is now a tuple of 1",
            "f output body: tuple of 2 is now a tuple of 1",
        ],
    );

    let (old, new) = both_sides(
        vec![tuple(vec!["u32", "u32"])],
        vec![tuple(vec!["u32", "u64"])],
    );
    assert_eq!(
        breaking(&old, &new),
        vec!["f output body.1: changed from `u32` to `u64`"]
    );
}

#[test]
fn container_items_and_arity_are_compared() {
    let vec_of = |item: &str| json!({ "name": "std::vec::Vec", "arguments": [reference(item)] });
    let (old, new) = both_sides(
        vec![single_field(vec_of("u32"))],
        vec![single_field(vec_of("u64"))],
    );
    assert_eq!(
        breaking(&old, &new),
        vec!["f output body.a[]: changed from `u32` to `u64`"]
    );

    let map = json!({
        "kind": "primitive",
        "name": "std::collections::HashMap",
        "parameters": [{ "name": "K" }, { "name": "V" }],
    });
    let map_of = |key: &str, value: &str| json!({ "name": "std::collections::HashMap", "arguments": [reference(key), reference(value)] });
    let (old, new) = both_sides(
        vec![
            map.clone(),
            single_field(map_of("std::string::String", "u32")),
        ],
        vec![map, single_field(map_of("u32", "u32"))],
    );
    assert_eq!(
        breaking(&old, &new),
        vec![
            "f input body.a<0>: changed from `std::string::String` to `u32`",
            "f output body.a<0>: changed from `std::string::String` to `u32`",
        ],
    );
}

#[test]
fn recursive_types_terminate() {
    let node = |extra: Vec<Value>| {
        let children = field(
            "children",
            json!({ "name": "std::vec::Vec", "arguments": [reference("T")] }),
            true,
        );
        named("T", [vec![children], extra].concat())
    };
    let (old, new) = both_sides(
        vec![node(vec![])],
        vec![node(vec![field("label", reference("u32"), true)])],
    );
    assert_eq!(
        breaking(&old, &new),
        vec!["f input body.label: required field added"]
    );
}

#[test]
fn a_shared_type_is_reported_for_every_function_that_uses_it() {
    let uses = |name: &str| json!({ "name": name, "path": "/api", "output_type": reference("T") });
    let old = schema_with(
        vec![uses("f"), uses("g")],
        vec![single_field(reference("u32"))],
    );
    let new = schema_with(vec![uses("f"), uses("g")], vec![named("T", vec![])]);
    assert_eq!(
        breaking(&old, &new),
        vec![
            "f output body.a: required field removed",
            "g output body.a: required field removed"
        ],
    );
}

#[test]
fn removed_fields_and_variants_carry_their_deprecation() {
    let (old, new) = both_sides(
        vec![named(
            "T",
            vec![json!({
                "name": "a",
                "type": reference("u32"),
                "required": true,
                "deprecation_note": "",
                "sunset": "2026-01-01",
            })],
        )],
        vec![named("T", vec![])],
    );
    assert_eq!(
        breaking(&old, &new),
        vec!["f output body.a: required field removed (deprecated, sunset 2026-01-01)"],
    );
}

mod removals {
    use super::*;
    use reflectapi_schema::diff::{diff_with, Options};

    fn options(today: &str, min_notice_days: Option<u32>) -> Options {
        Options {
            today: Some(today.parse().unwrap()),
            min_notice_days,
            ..Options::default()
        }
    }

    fn deprecation(since: Option<&str>, sunset: Option<&str>) -> Value {
        let mut d = json!({ "deprecation_note": "use g" });
        if let Some(since) = since {
            d["deprecated_since"] = json!(since);
        }
        if let Some(sunset) = sunset {
            d["sunset"] = json!(sunset);
        }
        d
    }

    fn remove_function(deprecated: Option<Value>, options: &Options) -> Vec<String> {
        let mut f = function(json!({}));
        if let Some(deprecated) = deprecated {
            f.as_object_mut()
                .unwrap()
                .extend(deprecated.as_object().unwrap().clone());
        }
        let old = schema_with(vec![f], vec![]);
        let new = schema_with(vec![], vec![]);
        diff_with(&old, &new, options)
            .into_iter()
            .map(|c| format!("{:?} {c}", c.severity))
            .collect()
    }

    #[test]
    fn without_a_date_every_removal_breaks() {
        assert_eq!(
            remove_function(
                Some(deprecation(None, Some("2026-01-01"))),
                &Options::default()
            ),
            vec!["Breaking f: function removed (deprecated, sunset 2026-01-01)"],
        );
    }

    #[test]
    fn a_removal_is_compatible_from_its_sunset_on() {
        let today = options("2026-11-20", None);
        let case = |deprecated| remove_function(deprecated, &today);

        assert_eq!(
            case(None),
            vec!["Breaking f: function removed without deprecation"]
        );
        assert_eq!(
            case(Some(deprecation(None, None))),
            vec!["Breaking f: function removed, deprecated without a sunset"],
        );
        assert_eq!(
            case(Some(deprecation(None, Some("2026-11-21")))),
            vec!["Breaking f: function removed before its sunset 2026-11-21"],
        );
        assert_eq!(
            case(Some(deprecation(None, Some("2026-11-20")))),
            vec!["Compatible f: function removed after its sunset 2026-11-20"],
        );
        assert_eq!(
            case(Some(deprecation(None, Some("2026-13-01")))),
            vec!["Breaking f: function removed, deprecated without a sunset"],
        );
    }

    #[test]
    fn min_notice_counts_from_the_deprecation_date() {
        let today = options("2026-11-20", Some(7));
        let case = |since| remove_function(Some(deprecation(since, Some("2026-11-20"))), &today);

        assert_eq!(
            case(None),
            vec!["Breaking f: function removed, deprecated without a date, 7 days' notice needed"],
        );
        assert_eq!(
            case(Some("2026-11-14")),
            vec![
                "Breaking f: function removed 6 days after its deprecation on 2026-11-14, 7 needed"
            ],
        );
        assert_eq!(
            case(Some("2026-11-13")),
            vec!["Compatible f: function removed after its sunset 2026-11-20"],
        );
        assert_eq!(
            case(Some("2025-12-25")),
            vec!["Compatible f: function removed after its sunset 2026-11-20"],
        );
    }

    #[test]
    fn fields_and_variants_follow_the_same_rule() {
        let deprecated_field = |name: &str| {
            let mut f = field(name, reference("u32"), true);
            f.as_object_mut().unwrap().extend(
                deprecation(Some("2026-10-01"), Some("2026-11-01"))
                    .as_object()
                    .unwrap()
                    .clone(),
            );
            f
        };
        let mut deprecated_variant = json!({ "name": "b", "fields": "none" });
        deprecated_variant.as_object_mut().unwrap().extend(
            deprecation(Some("2026-10-01"), Some("2026-12-01"))
                .as_object()
                .unwrap()
                .clone(),
        );
        let enumeration =
            |variants: Vec<Value>| json!({ "kind": "enum", "name": "E", "variants": variants });
        let unit = json!({ "name": "a", "fields": "none" });

        let (old, new) = both_sides(
            vec![
                enumeration(vec![unit.clone(), deprecated_variant]),
                named(
                    "T",
                    vec![deprecated_field("a"), field("e", reference("E"), true)],
                ),
            ],
            vec![
                enumeration(vec![unit]),
                named("T", vec![field("e", reference("E"), true)]),
            ],
        );
        let changes: Vec<String> = diff_with(&old, &new, &options("2026-11-20", Some(30)))
            .into_iter()
            .map(|c| format!("{:?} {c}", c.severity))
            .collect();
        assert_eq!(
            changes,
            vec![
                "Breaking f input body.e::b: variant removed before its sunset 2026-12-01",
                "Compatible f input body.a: field removed, the server ignores it if sent (deprecated, sunset 2026-11-01)",
                "Compatible f output body.a: required field removed after its sunset 2026-11-01",
                "Compatible f output body.e::b: variant removed (deprecated, sunset 2026-12-01)",
            ],
        );
    }

    fn with_deprecation(mut item: Value, since: &str, sunset: &str) -> Value {
        item.as_object_mut().unwrap().extend(
            deprecation(Some(since), Some(sunset))
                .as_object()
                .unwrap()
                .clone(),
        );
        item
    }

    #[test]
    fn a_removed_function_must_have_been_deprecated_in_the_earlier_schema() {
        let deprecated = with_deprecation(function(json!({})), "2026-10-01", "2026-11-01");
        let base = schema_with(vec![deprecated.clone()], vec![]);
        let head = schema_with(vec![], vec![]);
        let case = |snapshot: Schema| -> Vec<String> {
            let options = Options {
                deprecated_in: Some(snapshot),
                ..options("2026-11-20", None)
            };
            diff_with(&base, &head, &options)
                .into_iter()
                .map(|c| format!("{:?} {c}", c.severity))
                .collect()
        };

        assert_eq!(
            case(schema_with(vec![deprecated], vec![])),
            vec!["Compatible f: function removed after its sunset 2026-11-01"],
        );
        assert_eq!(
            case(schema_with(vec![function(json!({}))], vec![])),
            vec!["Breaking f: function removed, not deprecated in the earlier schema"],
        );
        assert_eq!(
            case(schema_with(vec![], vec![])),
            vec!["Breaking f: function removed, not deprecated in the earlier schema"],
        );
    }

    #[test]
    fn removed_fields_and_variants_must_have_been_deprecated_in_the_earlier_schema() {
        let types = |deprecated: bool, removed: bool| {
            let mark = |item: Value| {
                if deprecated {
                    with_deprecation(item, "2026-10-01", "2026-11-01")
                } else {
                    item
                }
            };
            let mut variants = vec![json!({ "name": "a", "fields": "none" })];
            let mut fields = vec![field("e", reference("E"), true)];
            if !removed {
                variants.push(mark(json!({ "name": "b", "fields": "none" })));
                fields.push(mark(field("x", reference("u32"), true)));
            }
            vec![
                json!({ "kind": "enum", "name": "E", "variants": variants }),
                named("T", fields),
            ]
        };
        let f = function(json!({ "input_type": reference("T"), "output_type": reference("T") }));
        let base = schema_with(vec![f.clone()], types(true, false));
        let head = schema_with(vec![f.clone()], types(true, true));
        let breaking_with = |snapshot: Schema| -> Vec<String> {
            let options = Options {
                deprecated_in: Some(snapshot),
                ..options("2026-11-20", None)
            };
            diff_with(&base, &head, &options)
                .into_iter()
                .filter(|c| c.severity == reflectapi_schema::diff::Severity::Breaking)
                .map(|c| c.to_string())
                .collect()
        };

        assert_eq!(
            breaking_with(schema_with(vec![f.clone()], types(true, false))),
            Vec::<String>::new(),
        );
        assert_eq!(
            breaking_with(schema_with(vec![f], types(false, false))),
            vec![
                "f input body.e::b: variant removed, not deprecated in the earlier schema",
                "f output body.x: required field removed, not deprecated in the earlier schema",
            ],
        );
    }

    #[test]
    fn a_route_added_since_the_earlier_schema_may_remove_a_field_deprecated_on_its_type() {
        let pet = |with_x: bool| {
            let mut fields = vec![field("name", reference("u32"), true)];
            if with_x {
                fields.push(with_deprecation(
                    field("x", reference("u32"), true),
                    "2026-10-01",
                    "2026-11-01",
                ));
            }
            named("Pet", fields)
        };
        let route =
            |name: &str| json!({ "name": name, "path": "/api", "output_type": reference("Pet") });

        let snapshot = schema_with(vec![route("old.route")], vec![pet(true)]);
        let base = schema_with(
            vec![route("old.route"), route("new.route")],
            vec![pet(true)],
        );
        let head = schema_with(
            vec![route("old.route"), route("new.route")],
            vec![pet(false)],
        );
        let options = Options {
            deprecated_in: Some(snapshot),
            ..options("2026-11-20", None)
        };
        let changes: Vec<String> = diff_with(&base, &head, &options)
            .into_iter()
            .map(|c| format!("{:?} {c}", c.severity))
            .collect();
        assert_eq!(
            changes,
            vec![
                "Compatible new.route output body.x: required field removed after its sunset 2026-11-01",
                "Compatible old.route output body.x: required field removed after its sunset 2026-11-01",
            ],
        );
    }

    #[test]
    fn json_carries_the_deprecation_dates() {
        let old = schema_with(
            vec![{
                let mut f = function(json!({}));
                f.as_object_mut().unwrap().extend(
                    deprecation(Some("2026-10-01"), Some("2026-11-01"))
                        .as_object()
                        .unwrap()
                        .clone(),
                );
                f
            }],
            vec![],
        );
        let new = schema_with(vec![], vec![]);
        let changes = diff_with(&old, &new, &options("2026-11-20", Some(7)));
        assert_eq!(
            serde_json::to_value(&changes).unwrap(),
            json!([{
                "severity": "compatible",
                "kind": "function-removed",
                "function": "f",
                "message": "function removed after its sunset 2026-11-01",
                "key": "f [function-removed]",
                "was_deprecated": true,
                "deprecated_since": "2026-10-01",
                "sunset": "2026-11-01",
            }]),
        );
    }
}

#[test]
fn tags_choose_the_functions_compared() {
    use reflectapi_schema::diff::{diff_with, Options};
    use std::collections::BTreeSet;

    let tagged = |name: &str, tags: &[&str]| json!({ "name": name, "path": "/api", "tags": tags });
    let old = schema_with(
        vec![
            tagged("public", &["Pets"]),
            tagged("internal", &["Pets", "Internal"]),
            tagged("other", &["Orders"]),
        ],
        vec![],
    );
    let new = schema_with(vec![], vec![]);
    let removed = |options: Options| -> Vec<String> {
        diff_with(&old, &new, &options)
            .into_iter()
            .map(|c| c.function)
            .collect()
    };
    let tags = |tags: &[&str]| tags.iter().map(|t| t.to_string()).collect::<BTreeSet<_>>();

    assert_eq!(
        removed(Options::default()),
        vec!["internal", "other", "public"]
    );
    assert_eq!(
        removed(Options {
            exclude_tags: tags(&["Internal"]),
            ..Options::default()
        }),
        vec!["other", "public"],
    );
    assert_eq!(
        removed(Options {
            include_tags: tags(&["Pets"]),
            exclude_tags: tags(&["Internal"]),
            ..Options::default()
        }),
        vec!["public"],
    );
}

#[test]
fn a_request_may_leave_out_an_option_unless_it_is_deserialized_by_hand() {
    let with = |extra: Value| named("T", vec![field("a", reference("u32"), true), extra]);
    let (old, new) = both_sides(
        vec![with(field("b", reference("u32"), true))],
        vec![with(field("c", optional(reference("u32")), true))],
    );
    assert_eq!(
        all(&old, &new),
        vec![
            "Breaking f output body.b: required field removed",
            "Compatible f input body.b: field removed, the server ignores it if sent",
            "Compatible f input body.c: optional field added",
            "Compatible f output body.c: field added",
        ],
    );

    let mut by_hand = field("c", optional(reference("u32")), true);
    by_hand["deserialize_with"] = json!(true);
    let (old, new) = both_sides(vec![named("T", vec![])], vec![named("T", vec![by_hand])]);
    assert_eq!(
        breaking(&old, &new),
        vec!["f input body.c: required field added"]
    );
}

mod acknowledgements {
    use super::*;
    use reflectapi_schema::diff::{acknowledge, parse_acknowledgements, Kind};

    fn removed_required_and_added_optional() -> Vec<reflectapi_schema::diff::Change> {
        let (old, new) = both_sides(
            vec![named("T", vec![field("a", reference("u32"), true)])],
            vec![named(
                "T",
                vec![field("b", optional(reference("u32")), false)],
            )],
        );
        diff(&old, &new)
    }

    #[test]
    fn every_kind_reads_back_from_its_name() {
        for kind in Kind::ALL {
            assert_eq!(kind.as_str().parse::<Kind>(), Ok(*kind));
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(kind.as_str()));
        }
        assert!("required-field-gone".parse::<Kind>().is_err());
    }

    #[test]
    fn a_key_names_function_location_and_kind() {
        let keys: Vec<String> = removed_required_and_added_optional()
            .iter()
            .map(|c| c.key.clone())
            .collect();
        assert_eq!(
            keys,
            vec![
                "f output body.a [required-field-removed]",
                "f input body.a [field-removed]",
                "f input body.b [field-added]",
                "f output body.b [field-added]",
            ],
        );
        let old = schema_with(vec![function(json!({}))], vec![]);
        let new = schema_with(vec![], vec![]);
        assert_eq!(diff(&old, &new)[0].key, "f [function-removed]");
    }

    #[test]
    fn lines_are_read_with_their_source() {
        let text = "# Removing `a`\n\nChecked with the web team.\n\n- f output body.items[].a [required-field-removed]: no client reads it, PETS-12\n-   f [function-removed] :  replaced by g  \n";
        let acks = parse_acknowledgements(text, "acks/one.md").unwrap();
        let read: Vec<(&str, &str, &str)> = acks
            .iter()
            .map(|a| (a.key.as_str(), a.reason.as_str(), a.source.as_str()))
            .collect();
        assert_eq!(
            read,
            vec![
                (
                    "f output body.items[].a [required-field-removed]",
                    "no client reads it, PETS-12",
                    "acks/one.md:5",
                ),
                ("f [function-removed]", "replaced by g", "acks/one.md:6"),
            ],
        );
    }

    #[test]
    fn unreadable_lines_are_errors() {
        let problem = |line: &str| parse_acknowledgements(line, "acks.md").unwrap_err().problem;
        let shape = "expected `- <function> <location> [<kind>]: <reason>`";
        assert_eq!(problem("- f output body.a: no reason to doubt"), shape);
        assert_eq!(
            problem("- f output body.a [required-field-gone]: typo"),
            shape
        );
        let reason = "give the reason this break is safe";
        assert_eq!(
            problem("- f output body.a [required-field-removed]"),
            reason
        );
        assert_eq!(
            problem("- f output body.a [required-field-removed]:  "),
            reason
        );
        assert_eq!(
            problem(
                "- f output body.a [required-field-removed]: <reason, consumers checked, ticket>"
            ),
            reason,
        );
    }

    #[test]
    fn only_breaking_changes_are_acknowledged_and_unused_lines_are_returned() {
        let mut changes = removed_required_and_added_optional();
        let acks = parse_acknowledgements(
            "- f output body.a [required-field-removed]: unused since v2\n\
             - f input body.b [field-added]: not a break\n\
             - g [function-removed]: another route\n",
            "acks.md",
        )
        .unwrap();
        let unused: Vec<String> = acknowledge(&mut changes, &acks)
            .into_iter()
            .map(|a| a.key)
            .collect();
        assert_eq!(
            unused,
            vec!["f input body.b [field-added]", "g [function-removed]"]
        );
        let acknowledged: Vec<(String, Option<String>)> = changes
            .into_iter()
            .map(|c| (c.key, c.acknowledged))
            .filter(|(_, a)| a.is_some())
            .collect();
        assert_eq!(
            acknowledged,
            vec![(
                "f output body.a [required-field-removed]".to_string(),
                Some("unused since v2".to_string()),
            )],
        );
    }
}

#[test]
fn warn_downgrades_only_breaking_changes_of_its_kinds() {
    use reflectapi_schema::diff::{diff_with, Kind, Options};

    let enumeration = |variants: &[&str]| {
        let variants: Vec<Value> = variants
            .iter()
            .map(|name| json!({ "name": name, "fields": "none" }))
            .collect();
        json!({ "kind": "enum", "name": "E", "variants": variants })
    };
    let (old, new) = both_sides(
        vec![
            enumeration(&["a"]),
            named(
                "T",
                vec![
                    field("e", reference("E"), true),
                    field("x", reference("u32"), true),
                ],
            ),
        ],
        vec![
            enumeration(&["a", "b"]),
            named("T", vec![field("e", reference("E"), true)]),
        ],
    );
    let options = Options {
        warn: [Kind::VariantAdded].into(),
        ..Options::default()
    };
    let changes: Vec<String> = diff_with(&old, &new, &options)
        .into_iter()
        .map(|c| format!("{:?} {c}", c.severity))
        .collect();
    assert_eq!(
        changes,
        vec![
            "Breaking f output body.x: required field removed",
            "Warning f output body.e::b: variant added, old clients can't read it",
            "Compatible f input body.e::b: variant added",
            "Compatible f input body.x: field removed, the server ignores it if sent",
        ],
    );
}

mod flatten {
    use super::*;

    fn internally_tagged(name: &str, tag: &str, variant: &str, field_name: &str) -> Value {
        json!({
            "kind": "enum",
            "name": name,
            "representation": { "internal": { "tag": tag } },
            "variants": [{ "name": variant, "fields": { "named": [field(field_name, reference("u32"), true)] } }],
        })
    }

    fn flattened(name: &str, ty: Value) -> Value {
        json!({ "name": name, "type": ty, "required": true, "flattened": true })
    }

    #[test]
    fn renaming_a_flattened_field_changes_nothing_on_the_wire() {
        let e = internally_tagged("E", "kind", "a", "x");
        let g = internally_tagged("G", "type", "b", "y");
        let (old, new) = both_sides(
            vec![
                e.clone(),
                g.clone(),
                named(
                    "T",
                    vec![
                        flattened("e", reference("E")),
                        flattened("g", reference("G")),
                    ],
                ),
            ],
            vec![
                e,
                g,
                named(
                    "T",
                    vec![
                        flattened("event", reference("E")),
                        flattened("group", reference("G")),
                    ],
                ),
            ],
        );
        assert_eq!(all(&old, &new), Vec::<String>::new());
    }

    #[test]
    fn the_one_flattened_value_left_on_each_side_is_compared() {
        let (old, new) = both_sides(
            vec![
                internally_tagged("E", "kind", "a", "x"),
                named("T", vec![flattened("e", reference("E"))]),
            ],
            vec![
                internally_tagged("E2", "kind", "a", "x"),
                named("T", vec![flattened("event", reference("E2"))]),
            ],
        );
        assert_eq!(
            all(&old, &new),
            vec![
                "Compatible f input body.(flattened e): type `E` is now `E2`",
                "Compatible f output body.(flattened e): type `E` is now `E2`",
            ],
        );
    }

    #[test]
    fn a_flattened_map_may_be_empty() {
        let map = json!({
            "kind": "primitive",
            "name": "std::collections::HashMap",
            "parameters": [{ "name": "K" }, { "name": "V" }],
        });
        let extra = flattened(
            "extra",
            json!({ "name": "std::collections::HashMap", "arguments": [reference("std::string::String"), reference("u32")] }),
        );
        let (old, new) = both_sides(
            vec![
                map.clone(),
                named("T", vec![field("id", reference("u32"), true)]),
            ],
            vec![
                map,
                named("T", vec![field("id", reference("u32"), true), extra]),
            ],
        );
        assert_eq!(breaking(&old, &new), Vec::<String>::new());
    }

    #[test]
    fn an_enum_wrapped_with_another_flattened_enum_gains_its_fields() {
        let e = internally_tagged("E", "kind", "a", "x");
        let g = internally_tagged("G", "type", "b", "y");
        let wrapper = named(
            "W",
            vec![
                flattened("e", reference("E")),
                flattened("g", reference("G")),
            ],
        );
        let (old, new) = both_sides(
            vec![e.clone(), g.clone(), single_field(reference("E"))],
            vec![e, g, wrapper, single_field(reference("W"))],
        );
        assert_eq!(
            all(&old, &new),
            vec![
                "Breaking f input body.a.(flattened g): required field added",
                "Compatible f output body.a.(flattened g): field added",
            ],
        );
    }
}
