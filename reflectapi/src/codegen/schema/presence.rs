//! Field wire-contract resolution.
//!
//! A field's wire behavior decomposes into three orthogonal facts, resolved
//! here once so that every backend renders from the same meaning instead of
//! re-deriving it from `(required, type name, custom_codec)` heuristics:
//!
//! - **key presence** — may the key be absent on the wire?
//! - **value nullability** — may the value be `null`?
//! - **absence semantics** — is "absent" distinct from "null"?
//!
//! The resolution rules, with the serde behavior that justifies them:
//!
//! | Field shape | Missing key behavior (deserialize) |
//! |---|---|
//! | `T` | rejected — key required |
//! | `Option<T>`, no attrs | accepted as `None` — `missing_field` special-cases `deserialize_option` |
//! | `reflectapi::Option<T>`, no `default` | accepted as `None` (collapses the three-state type; a lint may catch this in future) |
//! | `reflectapi::Option<T>` + `default` | accepted as `Undefined` |
//! | any + `serde(default)` | accepted — default value used |
//! | option + `with`/`deserialize_with`, no `default` | **rejected** — `missing_field` cannot route through a custom codec |
//!
//! On the serialize side the folded `required` flag (false when
//! `skip_serializing_if` is present) already captures whether the key can be
//! absent, and option types capture nullability.

use reflectapi_schema::Field;

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
    /// The value may be `null` on the wire (option-typed field).
    pub nullable: bool,
    /// Absence carries distinct meaning from an explicit `null`
    /// (three-state `reflectapi::Option`).
    pub absent_distinct_from_null: bool,
}

/// Resolve a field's wire contract.
///
/// `effective_required` is the folded serde requiredness for the rendering
/// context — usually `field.required`, but callers expanding flattened
/// fields combine it with the parent's requiredness.
pub(crate) fn resolve_field_wire_contract(
    field: &Field,
    effective_required: bool,
) -> FieldWireContract {
    let is_std_option = field.type_ref.name == "std::option::Option";
    let is_reflect_option = field.type_ref.name == "reflectapi::Option";
    let is_option = is_std_option || is_reflect_option;

    let key = if !effective_required {
        // serde(default) / skip_serializing_if: absence is always fine,
        // custom codec or not (a default is applied without invoking it).
        KeyPresence::Optional
    } else if is_option && !field.custom_codec {
        // serde accepts a missing key for plain option-typed fields even
        // without serde(default); a custom codec breaks that path, so
        // `required` stays authoritative there.
        KeyPresence::Optional
    } else {
        KeyPresence::Required
    };

    FieldWireContract {
        key,
        nullable: is_option,
        absent_distinct_from_null: is_reflect_option,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reflectapi_schema::TypeReference;

    fn field(type_name: &str, required: bool, custom_codec: bool) -> Field {
        let argument = TypeReference::new("u8".to_string(), vec![]);
        let mut f = Field::new(
            "f".to_string(),
            TypeReference::new(type_name.to_string(), vec![argument]),
        );
        f.required = required;
        f.custom_codec = custom_codec;
        f
    }

    #[test]
    fn plain_required_field_is_required() {
        let c = resolve_field_wire_contract(&field("u8", true, false), true);
        assert_eq!(c.key, KeyPresence::Required);
        assert!(!c.nullable);
        assert!(!c.absent_distinct_from_null);
    }

    #[test]
    fn std_option_is_optional_even_when_required() {
        let c = resolve_field_wire_contract(&field("std::option::Option", true, false), true);
        assert_eq!(c.key, KeyPresence::Optional);
        assert!(c.nullable);
        assert!(!c.absent_distinct_from_null);
    }

    #[test]
    fn reflectapi_option_is_optional_and_three_state() {
        let c = resolve_field_wire_contract(&field("reflectapi::Option", true, false), true);
        assert_eq!(c.key, KeyPresence::Optional);
        assert!(c.nullable);
        assert!(c.absent_distinct_from_null);
    }

    #[test]
    fn custom_codec_keeps_required_authoritative() {
        let c = resolve_field_wire_contract(&field("std::option::Option", true, true), true);
        assert_eq!(c.key, KeyPresence::Required);
        assert!(c.nullable);
    }

    #[test]
    fn custom_codec_with_serde_default_is_optional() {
        let c = resolve_field_wire_contract(&field("std::option::Option", false, true), false);
        assert_eq!(c.key, KeyPresence::Optional);
    }

    #[test]
    fn non_required_non_option_is_optional_not_nullable() {
        let c = resolve_field_wire_contract(&field("u8", false, false), false);
        assert_eq!(c.key, KeyPresence::Optional);
        assert!(!c.nullable);
    }
}
