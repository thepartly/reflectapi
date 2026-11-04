//! Whether serde reads JSON `null` into a type.
//!
//! A field that may be omitted but whose type rejects `null` has two wire
//! states, absent or a value. Clients that represent "absent" with a null
//! sentinel must not send that sentinel, so backends ask here.

use std::collections::BTreeSet;

use reflectapi_schema::{Representation, Schema, Type, TypeReference};

use super::shape::Shape;

/// `Some(false)` when the type is defined in the schema as a struct or enum
/// that serde never reads from `null`, `Some(true)` when it may be, and
/// `None` when the schema doesn't define it (a primitive, a generic parameter
/// or an unknown name), which leaves the decision to the caller.
pub(crate) fn defined_type_accepts_null(type_ref: &TypeReference, schema: &Schema) -> Option<bool> {
    let type_ref = unwrap_pointers(type_ref);
    match lookup(&type_ref.name, schema)? {
        Type::Primitive(_) => None,
        _ => Some(accepts_null(&type_ref, schema, &mut BTreeSet::new())),
    }
}

fn lookup<'a>(name: &str, schema: &'a Schema) -> Option<&'a Type> {
    schema
        .input_types
        .get_type(name)
        .or_else(|| schema.output_types.get_type(name))
}

/// `Box<T>`, `Rc<T>` and `Arc<T>` are written as the `T` they point to.
fn unwrap_pointers(type_ref: &TypeReference) -> TypeReference {
    let mut current = type_ref.clone();
    let mut seen = BTreeSet::new();
    while matches!(
        current.name.as_str(),
        "std::boxed::Box" | "std::rc::Rc" | "std::sync::Arc"
    ) && seen.insert(current.clone())
    {
        match current.arguments.first() {
            Some(inner) => current = inner.clone(),
            None => break,
        }
    }
    current
}

/// Primitives whose serde representation never includes `null`.
fn primitive_rejects_null(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "std::string::String"
            | "str"
            | "std::vec::Vec"
            | "std::collections::HashMap"
            | "std::collections::BTreeMap"
            | "std::collections::HashSet"
            | "std::collections::BTreeSet"
            | "indexmap::IndexMap"
            | "indexmap::IndexSet"
            | "uuid::Uuid"
    )
}

/// Conservative: anything not known to reject `null` is assumed to accept it,
/// so a client never refuses a value the server would read.
fn accepts_null(
    type_ref: &TypeReference,
    schema: &Schema,
    visiting: &mut BTreeSet<TypeReference>,
) -> bool {
    let type_ref = unwrap_pointers(type_ref);
    match type_ref.name.as_str() {
        "std::option::Option"
        | "reflectapi::Option"
        | "std::tuple::Tuple0"
        | "serde_json::Value" => return true,
        name if primitive_rejects_null(name) => return false,
        _ => {}
    }
    if !visiting.insert(type_ref.clone()) {
        return true;
    }
    let result = match lookup(&type_ref.name, schema) {
        None => true,
        // A primitive's fallback describes its representation, not what its
        // decoder accepts (`Weak<T>` falls back to `T` but reads null).
        Some(Type::Primitive(_)) => true,
        Some(Type::Struct(s)) => {
            let bind = |inner: &TypeReference| substitute(inner, s.parameters(), &type_ref);
            if s.transparent {
                s.fields
                    .iter()
                    .next()
                    .is_none_or(|field| accepts_null(&bind(&field.type_ref), schema, visiting))
            } else {
                match Shape::of(&s.fields) {
                    Shape::Unit => true,
                    Shape::Newtype(field) => accepts_null(&bind(&field.type_ref), schema, visiting),
                    Shape::Tuple(_) | Shape::Struct(_) => false,
                }
            }
        }
        // An untagged enum, or a tagged one's `#[serde(untagged)]` fallback
        // variants, read whatever a variant on its own reads.
        Some(Type::Enum(e)) => {
            let untagged = e.representation == Representation::None;
            e.variants
                .iter()
                .filter(|variant| untagged || variant.untagged)
                .any(|variant| match Shape::of(&variant.fields) {
                    Shape::Unit => true,
                    Shape::Newtype(field) => accepts_null(
                        &substitute(&field.type_ref, e.parameters(), &type_ref),
                        schema,
                        visiting,
                    ),
                    Shape::Tuple(_) | Shape::Struct(_) => false,
                })
        }
    };
    visiting.remove(&type_ref);
    result
}

/// Replace a definition's own type parameter with the argument the reference
/// supplies, so `Wrapper<Option<u32>>` reads its field as `Option<u32>`.
fn substitute<'a>(
    inner: &TypeReference,
    parameters: impl Iterator<Item = &'a reflectapi_schema::TypeParameter>,
    origin: &TypeReference,
) -> TypeReference {
    for (index, parameter) in parameters.enumerate() {
        if parameter.name() == inner.name {
            // An unbound parameter can be anything, including an Option.
            return origin
                .arguments
                .get(index)
                .cloned()
                .unwrap_or_else(|| TypeReference::from("serde_json::Value"));
        }
    }
    inner.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Schema {
        serde_json::from_value(serde_json::json!({
            "name": "nullability",
            "input_types": {"types": [
                {"kind": "primitive", "name": "u32"},
                {"kind": "primitive", "name": "std::option::Option", "parameters": [{"name": "T"}]},
                {"kind": "primitive", "name": "std::boxed::Box", "parameters": [{"name": "T"}], "fallback": {"name": "T"}},
                {"kind": "struct", "name": "Named", "fields": {"named": [{"name": "a", "type": {"name": "u32"}}]}},
                {"kind": "struct", "name": "Empty", "fields": {"named": []}},
                {"kind": "struct", "name": "Unit", "fields": "none"},
                {"kind": "struct", "name": "Pair", "fields": {"unnamed": [
                    {"name": "0", "type": {"name": "u32"}}, {"name": "1", "type": {"name": "u32"}}
                ]}},
                {"kind": "struct", "name": "Id", "fields": {"unnamed": [{"name": "0", "type": {"name": "u32"}}]}},
                {"kind": "struct", "name": "MaybeId", "fields": {"unnamed": [
                    {"name": "0", "type": {"name": "std::option::Option", "arguments": [{"name": "u32"}]}}
                ]}},
                {"kind": "struct", "name": "Wrap", "parameters": [{"name": "T"}], "transparent": true,
                 "fields": {"named": [{"name": "inner", "type": {"name": "T"}}]}},
                {"kind": "struct", "name": "Loop", "fields": {"unnamed": [{"name": "0", "type": {"name": "Loop"}}]}},
                {"kind": "enum", "name": "Tagged", "variants": [{"name": "A", "fields": "none"}]},
                {"kind": "enum", "name": "Internal", "representation": {"internal": {"tag": "t"}},
                 "variants": [{"name": "A", "fields": "none"}]},
                {"kind": "enum", "name": "UntaggedUnit", "representation": "none",
                 "variants": [{"name": "A", "fields": "none"}, {"name": "B", "fields": {"unnamed": [{"name": "0", "type": {"name": "u32"}}]}}]},
                {"kind": "primitive", "name": "std::sync::Weak", "parameters": [{"name": "T"}], "fallback": {"name": "T"}},
                {"kind": "primitive", "name": "serde_json::Value"},
                {"kind": "struct", "name": "NewOf", "parameters": [{"name": "T"}], "fields": {"unnamed": [{"name": "0", "type": {"name": "T"}}]}},
                {"kind": "enum", "name": "TaggedWithUntaggedUnit", "variants": [
                    {"name": "A", "fields": {"unnamed": [{"name": "0", "type": {"name": "u32"}}]}},
                    {"name": "Nothing", "fields": "none", "untagged": true}
                ]},
                {"kind": "enum", "name": "TaggedWithUntaggedValue", "representation": {"internal": {"tag": "t"}}, "variants": [
                    {"name": "A", "fields": "none"},
                    {"name": "Count", "fields": {"unnamed": [{"name": "0", "type": {"name": "u32"}}]}, "untagged": true}
                ]},
                {"kind": "enum", "name": "UntaggedValue", "representation": "none",
                 "variants": [{"name": "B", "fields": {"unnamed": [{"name": "0", "type": {"name": "u32"}}]}}]}
            ]}
        }))
        .unwrap()
    }

    fn accepts(type_ref: serde_json::Value) -> Option<bool> {
        let type_ref: TypeReference = serde_json::from_value(type_ref).unwrap();
        defined_type_accepts_null(&type_ref, &schema())
    }

    #[test]
    fn structs_and_tagged_enums_reject_null() {
        for name in [
            "Named",
            "Empty",
            "Pair",
            "Id",
            "Tagged",
            "Internal",
            "UntaggedValue",
            "TaggedWithUntaggedValue",
        ] {
            assert_eq!(
                accepts(serde_json::json!({"name": name})),
                Some(false),
                "{name}"
            );
        }
    }

    #[test]
    fn units_untagged_units_and_wrapped_options_accept_null() {
        for name in [
            "Unit",
            "MaybeId",
            "UntaggedUnit",
            "Loop",
            "TaggedWithUntaggedUnit",
        ] {
            assert_eq!(
                accepts(serde_json::json!({"name": name})),
                Some(true),
                "{name}"
            );
        }
    }

    #[test]
    fn transparent_and_pointer_wrappers_follow_what_they_wrap() {
        let wrap = |argument: serde_json::Value| {
            accepts(serde_json::json!({"name": "Wrap", "arguments": [argument]}))
        };
        assert_eq!(wrap(serde_json::json!({"name": "u32"})), Some(false));
        assert_eq!(
            wrap(
                serde_json::json!({"name": "std::option::Option", "arguments": [{"name": "u32"}]})
            ),
            Some(true)
        );
        assert_eq!(wrap(serde_json::json!({"name": "T"})), Some(true));
        assert_eq!(
            accepts(
                serde_json::json!({"name": "std::boxed::Box", "arguments": [{"name": "Named"}]})
            ),
            Some(false)
        );
    }

    #[test]
    fn a_newtype_follows_the_type_it_wraps() {
        let new_of = |argument: serde_json::Value| {
            accepts(serde_json::json!({"name": "NewOf", "arguments": [argument]}))
        };
        assert_eq!(new_of(serde_json::json!({"name": "Named"})), Some(false));
        assert_eq!(new_of(serde_json::json!({"name": "u32"})), Some(false));
        assert_eq!(
            new_of(serde_json::json!({"name": "serde_json::Value"})),
            Some(true)
        );
        assert_eq!(
            new_of(serde_json::json!({"name": "std::tuple::Tuple0"})),
            Some(true)
        );
        assert_eq!(
            new_of(
                serde_json::json!({"name": "std::sync::Weak", "arguments": [{"name": "Named"}]})
            ),
            Some(true)
        );
    }

    #[test]
    fn response_only_types_are_found() {
        let mut schema = schema();
        let named = schema.input_types.remove_type("Named").unwrap();
        schema.output_types.insert_type(named);
        let type_ref = TypeReference::from("Named");
        assert_eq!(defined_type_accepts_null(&type_ref, &schema), Some(false));
    }

    #[test]
    fn primitives_and_unknown_names_are_left_to_the_backend() {
        for name in ["u32", "std::option::Option", "T"] {
            assert_eq!(accepts(serde_json::json!({"name": name})), None, "{name}");
        }
    }
}
