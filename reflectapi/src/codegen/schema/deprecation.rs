//! Deprecated fields that a response may leave out.

use std::collections::HashSet;

use reflectapi_schema::{
    Field, Instantiate, Representation, Schema, Type, TypeReference, Typespace,
};

/// A field the transform made optional: its type, its variant if the type
/// is an enum, and its name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RelaxedField {
    pub type_name: String,
    pub variant: Option<String>,
    pub field: String,
}

/// A server removes a deprecated field once its sunset passes, so a client
/// generated before that must decode responses without it. Only types that
/// appear in responses alone are relaxed: a request type keeps the field
/// required, because the server may still require it until it is removed,
/// and so does every type a client decodes by trying it (see
/// `types_decoded_by_trying`). Every generator runs this before
/// consolidating the schema, so a type used in both directions is never
/// split by it, and all of them agree on which keys a response may leave
/// out. A relaxed type keeps its name through consolidation, so the returned
/// fields can be looked up in the consolidated schema.
pub(crate) fn relax_deprecated_response_fields(schema: &mut Schema) -> HashSet<RelaxedField> {
    let tried = types_decoded_by_trying(schema);
    let candidates: Vec<String> = schema
        .output_types
        .types()
        .map(|t| t.name().to_string())
        .filter(|name| {
            !schema.input_types.has_type(name)
                && !has_input_twin(schema, name)
                && !tried.contains(name)
        })
        .collect();
    let mut relaxed = HashSet::new();
    let mut relax = |type_name: &str, variant: Option<&str>, field: &mut Field| {
        if field.deprecation().is_some()
            && field.required
            && field.is_named()
            && !field.flattened
            && !field.deserialize_with
        {
            field.required = false;
            relaxed.insert(RelaxedField {
                type_name: type_name.to_owned(),
                variant: variant.map(str::to_owned),
                field: field.name.clone(),
            });
        }
    };
    for name in candidates {
        match schema.output_types.get_type_mut(&name) {
            Some(Type::Struct(s)) => s.fields.iter_mut().for_each(|f| relax(&name, None, f)),
            Some(Type::Enum(e)) => {
                for variant in &mut e.variants {
                    let variant_name = variant.name.clone();
                    variant
                        .fields
                        .iter_mut()
                        .for_each(|f| relax(&name, Some(&variant_name), f));
                }
            }
            _ => {}
        }
    }
    relaxed
}

/// Whether `name` is the response copy of a type that consolidation split
/// because its request and response shapes differ (`Schema::consolidate_types`
/// renames them `a::input::X` and `a::output::X`). Such a type is also sent
/// in requests, and clients often send back what they read, so it keeps its
/// deprecated fields required like any other type used in both directions.
fn has_input_twin(schema: &Schema, name: &str) -> bool {
    let mut parts: Vec<&str> = name.split("::").collect();
    match parts.len().checked_sub(2) {
        Some(module) if parts[module] == "output" => {
            parts[module] = "input";
            schema.input_types.has_type(&parts.join("::"))
        }
        _ => false,
    }
}

/// Types a client decodes by trying them in turn, and every type these
/// reach. A first-match decoder takes the first variant that matches, and a
/// flattened `Option` is `None` only when its type fails to decode, so an
/// optional field anywhere inside could change what a payload decodes as.
/// The seeds are untagged enums, enums with an untagged variant (a tagged
/// variant that fails falls back to it), and the type behind a flattened
/// `Option`. `Option` itself reads null or its argument, so it is no seed.
///
/// Generic types are walked as instances, with the arguments of each use
/// substituted into their fields, so `Either<Old, New>` reaches `Old` and a
/// generic struct flattening `Option<T>` reaches the `T` of each use.
fn types_decoded_by_trying(schema: &Schema) -> HashSet<String> {
    let types = &schema.output_types;
    let mut pending: Vec<TypeReference> = types
        .types()
        .filter(|ty| ty.parameters().next().is_none())
        .map(|ty| TypeReference::new(ty.name(), vec![]))
        .collect();
    for function in schema.functions() {
        pending.extend(function.output_type.type_refs().into_iter().cloned());
        pending.extend(function.error_type.iter().cloned());
    }

    // Every instance a response may contain, and the seeds among them.
    let mut instances = HashSet::new();
    let mut seeds = Vec::new();
    while let Some(type_ref) = pending.pop() {
        if !within_depth(&type_ref) || !instances.insert(type_ref.clone()) {
            continue;
        }
        pending.extend(type_ref.arguments.iter().cloned());
        let Some(instance) = Instance::of(types, &type_ref) else {
            continue;
        };
        if instance.tried_in_turn {
            seeds.push(type_ref.clone());
        }
        seeds.extend(flattened_options(instance.fields.iter()).cloned());
        pending.extend(instance.fields.into_iter().map(|f| f.type_ref));
    }

    let mut tried = HashSet::new();
    let mut visited = HashSet::new();
    let mut pending = seeds;
    while let Some(type_ref) = pending.pop() {
        if !within_depth(&type_ref) || !visited.insert(type_ref.clone()) {
            continue;
        }
        tried.insert(type_ref.name.clone());
        pending.extend(type_ref.arguments.iter().cloned());
        if let Some(instance) = Instance::of(types, &type_ref) {
            pending.extend(instance.fields.into_iter().map(|f| f.type_ref));
        }
    }
    tried
}

/// A type whose arguments nest its own name deeper on each step, such as
/// `struct Tree<T> { children: Vec<Tree<Vec<T>>> }`, has no finite set of
/// instances; the walk stops at this depth.
fn within_depth(type_ref: &TypeReference) -> bool {
    fn depth(type_ref: &TypeReference) -> usize {
        1 + type_ref.arguments.iter().map(depth).max().unwrap_or(0)
    }
    depth(type_ref) <= 16
}

/// The fields of a struct or enum with a use's arguments substituted in.
struct Instance {
    tried_in_turn: bool,
    fields: Vec<Field>,
}

impl Instance {
    fn of(types: &Typespace, type_ref: &TypeReference) -> Option<Self> {
        let ty = types.get_type(&type_ref.name)?;
        if ty.parameters().len() != type_ref.arguments.len() {
            return None;
        }
        match ty.clone() {
            Type::Struct(s) => Some(Instance {
                tried_in_turn: false,
                fields: s
                    .instantiate(&type_ref.arguments)
                    .fields
                    .iter()
                    .cloned()
                    .collect(),
            }),
            Type::Enum(e) => {
                let tried_in_turn = !is_option(&e.name)
                    && (e.representation == Representation::None
                        || e.variants.iter().any(|v| v.untagged));
                let e = e.instantiate(&type_ref.arguments);
                Some(Instance {
                    tried_in_turn,
                    fields: e
                        .variants
                        .iter()
                        .flat_map(|v| v.fields.iter().cloned())
                        .collect(),
                })
            }
            Type::Primitive(_) => None,
        }
    }
}

fn is_option(name: &str) -> bool {
    matches!(name, "std::option::Option" | "reflectapi::Option")
}

/// The types behind flattened `Option` fields.
fn flattened_options<'a>(
    fields: impl Iterator<Item = &'a Field>,
) -> impl Iterator<Item = &'a TypeReference> {
    fields
        .filter(|f| f.flattened && is_option(&f.type_ref.name))
        .flat_map(|f| f.type_ref.arguments.iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deprecated_fields_of_response_only_types_may_be_absent() {
        let field = |name: &str, deprecated: bool| {
            let mut field =
                serde_json::json!({"name": name, "required": true, "type": {"name": "u32"}});
            if deprecated {
                field["deprecation_note"] = "gone soon".into();
            }
            field
        };
        let mut schema: Schema = serde_json::from_value(serde_json::json!({
            "name": "deprecated-responses",
            "input_types": {"types": [
                {"kind": "primitive", "name": "u32"},
                {"kind": "struct", "name": "Both", "fields": {"named": [field("old", true)]}}
            ]},
            "output_types": {"types": [
                {"kind": "primitive", "name": "u32"},
                {"kind": "struct", "name": "Both", "fields": {"named": [field("old", true)]}},
                {"kind": "struct", "name": "Reply", "fields": {"named": [field("old", true), field("kept", false)]}},
                {"kind": "enum", "name": "Event", "variants": [
                    {"name": "Changed", "fields": {"named": [field("old", true), field("kept", false)]}}
                ]},
                {"kind": "enum", "name": "Untagged", "representation": "none", "variants": [
                    {"name": "Changed", "fields": {"named": [field("old", true), field("kept", false)]}},
                    {"name": "Wrapped", "fields": {"unnamed": [{"name": "0", "type": {"name": "Inner"}}]}}
                ]},
                {"kind": "struct", "name": "Inner", "fields": {"named": [
                    field("old", true),
                    {"name": "nested", "required": true, "type": {"name": "Nested"}}
                ]}},
                {"kind": "struct", "name": "Nested", "fields": {"named": [field("old", true)]}},
                {"kind": "enum", "name": "Fallback", "variants": [
                    {"name": "Known", "fields": {"named": [field("old", true)]}},
                    {"name": "Other", "untagged": true, "fields": {"unnamed": [{"name": "0", "type": {"name": "Unmatched"}}]}}
                ]},
                {"kind": "struct", "name": "Unmatched", "fields": {"named": [field("old", true)]}},
                {"kind": "struct", "name": "Holder", "fields": {"named": [
                    {"name": "group", "required": true, "flattened": true, "type": {"name": "std::option::Option", "arguments": [{"name": "Group"}]}}
                ]}},
                {"kind": "struct", "name": "Group", "fields": {"named": [field("old", true)]}},
                {"kind": "enum", "name": "WithFallback", "variants": [
                    {"name": "Tagged", "fields": {"unnamed": [{"name": "0", "type": {"name": "TaggedPayload"}}]}},
                    {"name": "Rest", "untagged": true, "fields": {"unnamed": [{"name": "0", "type": {"name": "u32"}}]}}
                ]},
                {"kind": "struct", "name": "TaggedPayload", "fields": {"named": [field("old", true)]}},
                {"kind": "enum", "name": "Either", "representation": "none",
                    "parameters": [{"name": "L"}, {"name": "R"}],
                    "variants": [
                        {"name": "Left", "fields": {"unnamed": [{"name": "0", "type": {"name": "L"}}]}},
                        {"name": "Right", "fields": {"unnamed": [{"name": "0", "type": {"name": "R"}}]}}
                    ]},
                {"kind": "struct", "name": "Lefty", "fields": {"named": [field("old", true)]}},
                {"kind": "struct", "name": "Holds", "parameters": [{"name": "T"}], "fields": {"named": [
                    {"name": "group", "required": true, "flattened": true, "type": {"name": "std::option::Option", "arguments": [{"name": "T"}]}}
                ]}},
                {"kind": "struct", "name": "Held", "fields": {"named": [field("old", true)]}},
                {"kind": "struct", "name": "User", "fields": {"named": [
                    {"name": "either", "required": true, "type": {"name": "std::vec::Vec", "arguments": [
                        {"name": "Either", "arguments": [{"name": "Lefty"}, {"name": "u32"}]}
                    ]}},
                    {"name": "holds", "required": true, "type": {"name": "Holds", "arguments": [{"name": "Held"}]}},
                    {"name": "config", "required": true, "type": {"name": "std::option::Option", "arguments": [{"name": "Configured"}]}}
                ]}},
                {"kind": "enum", "name": "std::option::Option", "representation": "none",
                    "parameters": [{"name": "T"}],
                    "variants": [
                        {"name": "None", "fields": "none"},
                        {"name": "Some", "fields": {"unnamed": [{"name": "0", "type": {"name": "T"}}]}}
                    ]},
                {"kind": "struct", "name": "Configured", "fields": {"named": [field("old", true)]}},
                {"kind": "struct", "name": "FnLeft", "fields": {"named": [field("old", true)]}},
                {"kind": "struct", "name": "Forward", "parameters": [{"name": "T"}], "fields": {"named": [
                    {"name": "either", "required": true, "type": {"name": "Either", "arguments": [{"name": "T"}, {"name": "u32"}]}}
                ]}},
                {"kind": "struct", "name": "Forwarded", "fields": {"named": [field("old", true)]}}
            ]},
            "functions": [
                {"name": "direct", "path": "", "output_kind": "complete",
                    "output_type": {"name": "Either", "arguments": [{"name": "FnLeft"}, {"name": "u32"}]},
                    "error_type": {"name": "Forward", "arguments": [{"name": "Forwarded"}]}}
            ]
        }))
        .unwrap();
        let relaxed = super::relax_deprecated_response_fields(&mut schema);
        assert!(relaxed.contains(&RelaxedField {
            type_name: "Event".into(),
            variant: Some("Changed".into()),
            field: "old".into(),
        }));
        let required = |ts: &reflectapi_schema::Typespace, ty: &str| -> Vec<(String, bool)> {
            let fields = match ts.get_type(ty).unwrap() {
                Type::Struct(s) => s.fields.iter().collect::<Vec<_>>(),
                Type::Enum(e) => e.variants[0].fields.iter().collect(),
                _ => unreachable!(),
            };
            fields
                .iter()
                .map(|f| (f.name.clone(), f.required))
                .collect()
        };
        assert_eq!(
            required(&schema.output_types, "Reply"),
            [("old".into(), false), ("kept".into(), true)]
        );
        // `Option` reads null or its argument, not by trying.
        assert_eq!(
            required(&schema.output_types, "Configured"),
            [("old".into(), false)]
        );
        assert_eq!(
            required(&schema.output_types, "Event"),
            [("old".into(), false), ("kept".into(), true)]
        );
        // Untagged variants match as before, down to the types they reach.
        assert_eq!(
            required(&schema.output_types, "Untagged"),
            [("old".into(), true), ("kept".into(), true)]
        );
        assert_eq!(
            required(&schema.output_types, "Inner"),
            [("old".into(), true), ("nested".into(), true)]
        );
        assert_eq!(
            required(&schema.output_types, "Nested"),
            [("old".into(), true)]
        );
        // A tagged enum's untagged variants, and the type behind a flattened
        // `Option`, are decoded by trying them too.
        // So are a tagged variant of an enum with an untagged fallback, and
        // the arguments of a generic untagged enum or of a generic type
        // flattening an `Option` of its parameter, wherever it is used,
        // including a function's response and error types and a generic
        // type forwarding its parameter into one.
        for name in [
            "FnLeft",
            "Forwarded",
            "Fallback",
            "Unmatched",
            "Group",
            "TaggedPayload",
            "Lefty",
            "Held",
        ] {
            assert_eq!(
                required(&schema.output_types, name),
                [("old".into(), true)],
                "{name}"
            );
        }
        assert_eq!(
            required(&schema.output_types, "Both"),
            [("old".into(), true)]
        );
        assert_eq!(
            required(&schema.input_types, "Both"),
            [("old".into(), true)]
        );
    }

    #[test]
    fn the_response_copy_of_a_split_type_keeps_deprecated_fields() {
        let old = serde_json::json!({"name": "old", "required": true, "type": {"name": "u32"}, "deprecation_note": "gone soon"});
        let mut schema: Schema = serde_json::from_value(serde_json::json!({
            "name": "split",
            "input_types": {"types": [
                {"kind": "primitive", "name": "u32"},
                {"kind": "struct", "name": "rules::input::Split", "fields": {"named": [old.clone()]}}
            ]},
            "output_types": {"types": [
                {"kind": "primitive", "name": "u32"},
                {"kind": "struct", "name": "rules::output::Split", "fields": {"named": [old.clone()]}},
                {"kind": "struct", "name": "replies::output::Lone", "fields": {"named": [old]}}
            ]}
        }))
        .unwrap();
        let relaxed = super::relax_deprecated_response_fields(&mut schema);
        // A client may send back the `output` copy it read as the `input`
        // twin; an `output` module without a twin is an ordinary response.
        assert_eq!(
            relaxed.into_iter().map(|f| f.type_name).collect::<Vec<_>>(),
            ["replies::output::Lone"]
        );
    }
}
