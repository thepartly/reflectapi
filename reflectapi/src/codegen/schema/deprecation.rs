//! Deprecated fields that a response may leave out.

use std::collections::HashSet;

use reflectapi_schema::{Field, Representation, Schema, Type, TypeReference, Typespace};

/// A server removes a deprecated field once its sunset passes, so a client
/// generated before that must decode responses without it. Only types that
/// appear in responses alone are relaxed: a request type keeps the field
/// required, because the server may still require it until it is removed,
/// and so does every type a client decodes by trying it (see
/// `types_matched_untagged`).
/// Every generator runs this before consolidating the schema, so a type used
/// in both directions is never split by it, and all of them agree on which
/// keys a response may leave out.
pub(crate) fn relax_deprecated_response_fields(schema: &mut Schema) {
    let matched_untagged = types_matched_untagged(&schema.output_types);
    let relaxed: Vec<String> = schema
        .output_types
        .types()
        .map(|t| t.name().to_string())
        .filter(|name| !schema.input_types.has_type(name) && !matched_untagged.contains(name))
        .collect();
    let relax = |field: &mut Field| {
        if field.deprecation().is_some()
            && field.required
            && field.is_named()
            && !field.flattened
            && !field.deserialize_with
        {
            field.required = false;
        }
    };
    for name in relaxed {
        match schema.output_types.get_type_mut(&name) {
            Some(Type::Struct(s)) => s.fields.iter_mut().for_each(relax),
            Some(Type::Enum(e)) => {
                for variant in &mut e.variants {
                    variant.fields.iter_mut().for_each(relax);
                }
            }
            _ => {}
        }
    }
}

/// Types a client decodes by trying them in turn: untagged enums, enums with
/// untagged variants, the type behind a flattened `Option`, and every type
/// these reach. A first-match decoder takes the first variant that matches,
/// and a flattened `Option` is `None` only when its type fails to decode, so
/// an optional field anywhere inside could change what a payload decodes as.
fn types_matched_untagged(types: &Typespace) -> HashSet<String> {
    let mut pending: Vec<&TypeReference> = Vec::new();
    let mut seen = HashSet::new();
    for ty in types.types() {
        match ty {
            Type::Enum(e) => {
                let untagged = e.representation == Representation::None;
                for variant in &e.variants {
                    if untagged || variant.untagged {
                        seen.insert(e.name.clone());
                        pending.extend(variant.fields.iter().map(|f| &f.type_ref));
                    }
                    pending.extend(flattened_options(variant.fields.iter()));
                }
            }
            Type::Struct(s) => pending.extend(flattened_options(s.fields.iter())),
            Type::Primitive(_) => {}
        }
    }
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
    seen
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
                {"kind": "struct", "name": "Group", "fields": {"named": [field("old", true)]}}
            ]}
        }))
        .unwrap();
        super::relax_deprecated_response_fields(&mut schema);
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
        for name in ["Fallback", "Unmatched", "Group"] {
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
