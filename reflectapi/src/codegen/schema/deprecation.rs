//! Deprecated fields that a response may leave out.

use std::collections::HashSet;

use reflectapi_schema::{Field, Representation, Schema, Type, TypeReference, Typespace};

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
    let tried = types_decoded_by_trying(&schema.output_types);
    let candidates: Vec<String> = schema
        .output_types
        .types()
        .map(|t| t.name().to_string())
        .filter(|name| !schema.input_types.has_type(name) && !tried.contains(name))
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

/// Types a client decodes by trying them in turn, and every type these
/// reach. A first-match decoder takes the first variant that matches, and a
/// flattened `Option` is `None` only when its type fails to decode, so an
/// optional field anywhere inside could change what a payload decodes as.
/// The seeds are untagged enums, enums with an untagged variant (a tagged
/// variant that fails falls back to it), and the type behind a flattened
/// `Option`. A generic type that is reached, or that flattens an `Option` of
/// its own parameter, reaches the type arguments of every use of it.
fn types_decoded_by_trying(types: &Typespace) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut pending: Vec<&TypeReference> = Vec::new();
    for ty in types.types() {
        let parameters: HashSet<&str> = ty.parameters().map(|p| p.name.as_str()).collect();
        let seed_flattened = |fields: &'_ reflectapi_schema::Fields, seen: &mut HashSet<String>| {
            for inner in flattened_options(fields.iter()) {
                if parameters.contains(inner.name.as_str()) {
                    seen.insert(ty.name().to_owned());
                }
            }
        };
        match ty {
            Type::Enum(e) => {
                if e.representation == Representation::None || e.variants.iter().any(|v| v.untagged)
                {
                    seen.insert(e.name.clone());
                    pending.extend(
                        e.variants
                            .iter()
                            .flat_map(|v| v.fields.iter().map(|f| &f.type_ref)),
                    );
                }
                for variant in &e.variants {
                    pending.extend(flattened_options(variant.fields.iter()));
                    seed_flattened(&variant.fields, &mut seen);
                }
            }
            Type::Struct(s) => {
                pending.extend(flattened_options(s.fields.iter()));
                seed_flattened(&s.fields, &mut seen);
            }
            Type::Primitive(_) => {}
        }
    }
    let uses: Vec<&TypeReference> = types
        .types()
        .flat_map(|ty| match ty {
            Type::Struct(s) => s.fields.iter().collect::<Vec<_>>(),
            Type::Enum(e) => e.variants.iter().flat_map(|v| v.fields.iter()).collect(),
            Type::Primitive(_) => Vec::new(),
        })
        .map(|f| &f.type_ref)
        .collect();
    loop {
        while let Some(type_ref) = pending.pop() {
            pending.extend(type_ref.arguments.iter());
            if !seen.insert(type_ref.name.clone()) {
                continue;
            }
            match types.get_type(&type_ref.name) {
                Some(Type::Struct(s)) => pending.extend(s.fields.iter().map(|f| &f.type_ref)),
                Some(Type::Enum(e)) => pending.extend(
                    e.variants
                        .iter()
                        .flat_map(|v| v.fields.iter().map(|f| &f.type_ref)),
                ),
                _ => {}
            }
        }
        // A use of a reached generic type, possibly nested in another
        // type's arguments, reaches its arguments.
        let mut stack = uses.clone();
        while let Some(type_ref) = stack.pop() {
            if seen.contains(&type_ref.name) {
                pending.extend(
                    type_ref
                        .arguments
                        .iter()
                        .filter(|argument| !seen.contains(&argument.name)),
                );
            }
            stack.extend(type_ref.arguments.iter());
        }
        if pending.is_empty() {
            return seen;
        }
    }
}

/// The types behind flattened `Option` fields.
fn flattened_options<'a>(
    fields: impl Iterator<Item = &'a Field>,
) -> impl Iterator<Item = &'a TypeReference> {
    fields
        .filter(|f| {
            f.flattened
                && matches!(
                    f.type_ref.name.as_str(),
                    "std::option::Option" | "reflectapi::Option"
                )
        })
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
                    {"name": "holds", "required": true, "type": {"name": "Holds", "arguments": [{"name": "Held"}]}}
                ]}}
            ]}
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
        // flattening an `Option` of its parameter, wherever it is used.
        for name in [
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
}
