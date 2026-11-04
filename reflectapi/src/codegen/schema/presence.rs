//! A field's wire contract: whether its key may be absent, and whether its
//! value may be null.
//!
//! Reflection resolves Serde's directional missing-key behavior into
//! `Field.required` before input/output types are consolidated. Renderers must
//! preserve that decision, including overrides made by schema transforms and
//! required nullable fields in manually constructed or older schemas.
//!
//! A `#[serde(flatten)]` field is the exception: it has no key of its own, so
//! key-level attributes such as `default` and `skip_serializing_if` don't
//! apply to it, and `required` doesn't describe it. Serde makes a flattened
//! group optional only when its type is an `Option`.
//!
//! Whether the value may be null depends on the type's definition, so the
//! contract is resolved with the typespace being rendered.

use reflectapi_schema::{Field, TypeReference};

use super::layout::Lookup;
use super::nullability::accepts_null;

#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldWireFacts<'a> {
    pub type_ref: &'a TypeReference,
    pub required: bool,
    pub named: bool,
    pub flattened: bool,
    /// A custom serializer or deserializer, which may write or read null for
    /// any type.
    pub custom_codec: bool,
}

impl<'a> FieldWireFacts<'a> {
    pub fn of(field: &'a Field) -> Self {
        Self {
            type_ref: &field.type_ref,
            required: field.required,
            named: field.is_named(),
            flattened: field.flattened,
            custom_codec: field.serialize_with || field.deserialize_with,
        }
    }

    pub fn with_required(self, required: bool) -> Self {
        Self { required, ..self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyPresence {
    Required,
    Optional,
    /// No key of its own: the field's type contributes its keys to the parent.
    /// `optional` is whether the whole group may be absent.
    Flattened {
        optional: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldWireContract {
    pub key: KeyPresence,
    // The declared Rust type admits null. Custom codecs may change its wire
    // representation; use a reflected type override for such codecs.
    pub declared_nullable: bool,
    /// Serde reads the value from `null`. Unlike `declared_nullable`, this
    /// follows the type's definition: a unit struct or a newtype of an
    /// `Option` reads null, and so does any field with a custom codec.
    pub value_accepts_null: bool,
    named: bool,
}

impl FieldWireContract {
    /// The field has three wire states: absent, null and set.
    pub fn preserves_absence(&self) -> bool {
        self.named && self.key == KeyPresence::Optional && self.declared_nullable
    }

    /// For a flattened field, whether its group must be present.
    pub fn flattened_group_required(&self) -> bool {
        matches!(self.key, KeyPresence::Flattened { optional: false })
    }
}

fn is_option(type_ref: &TypeReference) -> bool {
    matches!(
        type_ref.name.as_str(),
        "std::option::Option" | "reflectapi::Option"
    )
}

/// The key half of the contract, which needs no typespace.
pub(crate) fn key_presence(facts: FieldWireFacts<'_>) -> KeyPresence {
    if facts.flattened {
        KeyPresence::Flattened {
            optional: is_option(facts.type_ref),
        }
    } else if facts.required {
        KeyPresence::Required
    } else {
        KeyPresence::Optional
    }
}

pub(crate) fn resolve_field_wire_contract(
    facts: FieldWireFacts<'_>,
    lookup: Lookup<'_, '_>,
) -> FieldWireContract {
    FieldWireContract {
        key: key_presence(facts),
        declared_nullable: is_option(facts.type_ref),
        value_accepts_null: facts.custom_codec || accepts_null(facts.type_ref, lookup),
        named: facts.named,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_types(_: &str) -> Option<&'static reflectapi_schema::Type> {
        None
    }

    fn resolve(facts: FieldWireFacts<'_>) -> FieldWireContract {
        resolve_field_wire_contract(facts, &no_types)
    }

    fn facts(type_ref: &TypeReference, required: bool) -> FieldWireFacts<'_> {
        FieldWireFacts {
            type_ref,
            required,
            named: true,
            flattened: false,
            custom_codec: false,
        }
    }

    #[test]
    fn required_is_authoritative_independently_of_declared_nullability() {
        for type_name in ["u8", "std::option::Option", "reflectapi::Option"] {
            let type_ref = TypeReference::from(type_name);
            for required in [true, false] {
                let contract = resolve(facts(&type_ref, required));
                assert_eq!(contract.key == KeyPresence::Required, required);
                assert_eq!(contract.declared_nullable, type_name != "u8");
                assert_eq!(contract.value_accepts_null, type_name != "u8");
            }
        }
    }

    #[test]
    fn a_custom_codec_may_read_null_for_any_type() {
        let type_ref = TypeReference::from("u8");
        let contract = resolve(FieldWireFacts {
            custom_codec: true,
            ..facts(&type_ref, false)
        });
        assert!(contract.value_accepts_null);
        assert!(!contract.declared_nullable);
    }

    #[test]
    fn old_or_manual_required_nullable_schema_stays_required() {
        let mut field = Field::new("value".into(), "std::option::Option".into());
        field.required = true;
        assert_eq!(
            resolve(FieldWireFacts::of(&field)).key,
            KeyPresence::Required
        );
    }

    #[test]
    fn only_an_optional_nullable_named_key_preserves_absence() {
        let contract = |type_name: &str, required, named| {
            let type_ref = TypeReference::from(type_name);
            resolve(FieldWireFacts {
                named,
                ..facts(&type_ref, required)
            })
        };
        assert!(contract("std::option::Option", false, true).preserves_absence());
        assert!(!contract("std::option::Option", true, true).preserves_absence());
        assert!(!contract("u8", false, true).preserves_absence());
        assert!(!contract("std::option::Option", false, false).preserves_absence());
    }

    #[test]
    fn a_flattened_group_is_optional_only_when_its_type_is_an_option() {
        for type_name in ["std::option::Option", "reflectapi::Option", "Inner"] {
            let type_ref = TypeReference::from(type_name);
            // `required` describes a key, which a flattened field doesn't have.
            for required in [true, false] {
                let contract = resolve(FieldWireFacts {
                    flattened: true,
                    ..facts(&type_ref, required)
                });
                let optional = type_name != "Inner";
                assert_eq!(contract.key, KeyPresence::Flattened { optional });
                assert_eq!(contract.flattened_group_required(), !optional);
                assert!(!contract.preserves_absence());
            }
        }
    }
}
