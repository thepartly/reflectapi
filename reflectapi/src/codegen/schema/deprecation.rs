//! Deprecated fields that a response may leave out.

use reflectapi_schema::{Field, Representation, Schema, Type};

/// A server removes a deprecated field once its sunset passes, so a client
/// generated before that must decode responses without it. Only types that
/// appear in responses alone are relaxed: a request type keeps the field
/// required, because the server may still require it until it is removed.
/// Every generator runs this before consolidating the schema, so a type used
/// in both directions is never split by it, and all of them agree on which
/// keys a response may leave out.
pub(crate) fn relax_deprecated_response_fields(schema: &mut Schema) {
    let response_only: Vec<String> = schema
        .output_types
        .types()
        .map(|t| t.name().to_string())
        .filter(|name| !schema.input_types.has_type(name))
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
    for name in response_only {
        match schema.output_types.get_type_mut(&name) {
            Some(Type::Struct(s)) => s.fields.iter_mut().for_each(relax),
            // An untagged enum picks the first variant that matches, so an
            // optional field could make an earlier variant match where serde
            // would pick a later one.
            Some(Type::Enum(e)) if e.representation != Representation::None => {
                for variant in &mut e.variants {
                    variant.fields.iter_mut().for_each(relax);
                }
            }
            _ => {}
        }
    }
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
                    {"name": "Changed", "fields": {"named": [field("old", true), field("kept", false)]}}
                ]}
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
