//! Field wire-contract resolution.
//!
//! A field's wire behavior decomposes into three orthogonal facts, resolved
//! here once so that every backend renders from the same meaning instead of
//! re-deriving it from `(required, type name, deserialize_with)` heuristics:
//!
//! - **key presence** — may the key be absent on the wire?
//! - **declared nullability** — does the declared type admit `null`?
//! - **absence semantics** — is "absent" distinct from "null"?
//!
//! The key-presence rules, with the serde deserialize behavior that
//! justifies them:
//!
//! | Field shape | Missing key behavior (deserialize) |
//! |---|---|
//! | `T` | rejected — key required |
//! | `Option<T>`, no attrs | accepted as `None` — `missing_field` special-cases `deserialize_option` |
//! | `reflectapi::Option<T>`, no `default` | accepted as `None` (collapses the three-state type to two) |
//! | `reflectapi::Option<T>` + `default` | accepted as `Undefined` |
//! | any + `serde(default)` | accepted — default value used |
//! | option + `with`/`deserialize_with`, no `default` | **rejected** — `missing_field` cannot route through a custom deserializer |
//!
//! Only the deserialize side affects key presence: a custom *serializer*
//! never changes whether the key may be absent — `skip_serializing_if`,
//! already folded into `required`, controls that.

use reflectapi_schema::Field;

/// The minimal facts key-presence resolution depends on.
///
/// Constructed from a [`Field`] via [`FieldWireFacts::of`]; callers that
/// have no schema field (e.g. synthesized flattened-enum fields) construct
/// it directly, making explicit exactly which facts they assert.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldWireFacts<'a> {
    /// Fully qualified name of the field's declared type.
    pub type_name: &'a str,
    /// The field has a custom serde deserializer
    /// (`#[serde(deserialize_with)]` or `#[serde(with)]`).
    pub deserialize_with: bool,
    /// Folded serde requiredness for the rendering context: absence of
    /// `#[serde(default)]` on the deserialize side, absence of
    /// `skip_serializing_if` on the serialize side.
    pub required: bool,
}

impl<'a> FieldWireFacts<'a> {
    pub fn of(field: &'a Field) -> Self {
        Self {
            type_name: &field.type_ref.name,
            deserialize_with: field.deserialize_with,
            required: field.required,
        }
    }

    /// Override requiredness, e.g. when a flattened parent's own
    /// requiredness folds into its expanded fields.
    pub fn with_required(self, required: bool) -> Self {
        Self { required, ..self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyPresence {
    /// The key must be present on the wire.
    Required,
    /// The key may be omitted by the sender.
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldWireContract {
    pub key: KeyPresence,
    /// The declared type admits `null` (option-typed field). Whether `null`
    /// actually appears on the wire is direction-dependent: an output field
    /// with `skip_serializing_if` omits the key instead of emitting `null`.
    pub declared_nullable: bool,
    /// Absence carries distinct meaning from an explicit `null`
    /// (three-state `reflectapi::Option`).
    pub absent_distinct_from_null: bool,
}

pub(crate) fn resolve_field_wire_contract(facts: FieldWireFacts<'_>) -> FieldWireContract {
    let is_std_option = facts.type_name == "std::option::Option";
    let is_reflect_option = facts.type_name == "reflectapi::Option";
    let is_option = is_std_option || is_reflect_option;

    let key = if !facts.required {
        // serde(default) / skip_serializing_if: absence is always fine,
        // custom deserializer or not (a default is applied without
        // invoking it).
        KeyPresence::Optional
    } else if is_option && !facts.deserialize_with {
        // serde accepts a missing key for plain option-typed fields even
        // without serde(default); a custom deserializer breaks that path,
        // so `required` stays authoritative there.
        KeyPresence::Optional
    } else {
        KeyPresence::Required
    };

    FieldWireContract {
        key,
        declared_nullable: is_option,
        absent_distinct_from_null: is_reflect_option,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(type_name: &str, required: bool) -> FieldWireFacts<'_> {
        FieldWireFacts {
            type_name,
            deserialize_with: false,
            required,
        }
    }

    #[test]
    fn plain_required_field_is_required() {
        let c = resolve_field_wire_contract(facts("u8", true));
        assert_eq!(c.key, KeyPresence::Required);
        assert!(!c.declared_nullable);
        assert!(!c.absent_distinct_from_null);
    }

    #[test]
    fn std_option_is_optional_even_when_required() {
        let c = resolve_field_wire_contract(facts("std::option::Option", true));
        assert_eq!(c.key, KeyPresence::Optional);
        assert!(c.declared_nullable);
        assert!(!c.absent_distinct_from_null);
    }

    #[test]
    fn reflectapi_option_is_optional_and_three_state() {
        let c = resolve_field_wire_contract(facts("reflectapi::Option", true));
        assert_eq!(c.key, KeyPresence::Optional);
        assert!(c.declared_nullable);
        assert!(c.absent_distinct_from_null);
    }

    #[test]
    fn custom_deserializer_keeps_required_authoritative() {
        let c = resolve_field_wire_contract(FieldWireFacts {
            deserialize_with: true,
            ..facts("std::option::Option", true)
        });
        assert_eq!(c.key, KeyPresence::Required);
        assert!(c.declared_nullable);
    }

    #[test]
    fn custom_deserializer_with_serde_default_is_optional() {
        let c = resolve_field_wire_contract(FieldWireFacts {
            deserialize_with: true,
            ..facts("std::option::Option", false)
        });
        assert_eq!(c.key, KeyPresence::Optional);
    }

    #[test]
    fn non_required_non_option_is_optional_not_nullable() {
        let c = resolve_field_wire_contract(facts("u8", false));
        assert_eq!(c.key, KeyPresence::Optional);
        assert!(!c.declared_nullable);
    }

    #[test]
    fn facts_of_field_carries_deserialize_with_only() {
        // A custom serializer must not affect key presence; `of` therefore
        // only reads the deserialize-side flag.
        let mut field = reflectapi_schema::Field::new(
            "f".to_string(),
            reflectapi_schema::TypeReference::new("std::option::Option".to_string(), vec![]),
        );
        field.required = true;
        field.serialize_with = true;
        let c = resolve_field_wire_contract(FieldWireFacts::of(&field));
        assert_eq!(c.key, KeyPresence::Optional);

        field.deserialize_with = true;
        let c = resolve_field_wire_contract(FieldWireFacts::of(&field));
        assert_eq!(c.key, KeyPresence::Required);
    }
}
