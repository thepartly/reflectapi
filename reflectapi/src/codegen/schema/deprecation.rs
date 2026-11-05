//! Deprecated fields that a response may leave out.

use std::collections::HashSet;

use reflectapi_schema::{Field, Representation, Schema, Type, TypeReference, Typespace};

/// A server removes a deprecated field once its sunset passes, so a client
/// generated before that must decode responses without it. Only types that
/// appear in responses alone are relaxed: a request type keeps the field
/// required, because the server may still require it until it is removed,
/// and so does every type an untagged enum's variants reach.
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

/// Untagged enums and every type their variants reach. An untagged enum
/// picks the first variant that matches, so an optional field anywhere
/// inside a variant could make it match a payload that serde gives to a
/// later variant.
fn types_matched_untagged(types: &Typespace) -> HashSet<String> {
    let mut pending: Vec<&TypeReference> = Vec::new();
    let mut seen = HashSet::new();
    for ty in types.types() {
        if let Type::Enum(e) = ty {
            if e.representation == Representation::None {
                seen.insert(e.name.clone());
                pending.extend(
                    e.variants
                        .iter()
                        .flat_map(|v| v.fields.iter().map(|f| &f.type_ref)),
                );
            }
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
                {"kind": "struct", "name": "Nested", "fields": {"named": [field("old", true)]}}
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
