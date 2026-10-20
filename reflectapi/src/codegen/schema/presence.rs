//! Rendering facts for field presence and declared nullability.
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

use reflectapi_schema::Field;

#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldWireFacts<'a> {
    pub type_name: &'a str,
    pub required: bool,
    pub named: bool,
    pub flattened: bool,
}

impl<'a> FieldWireFacts<'a> {
    pub fn of(field: &'a Field) -> Self {
        Self {
            type_name: &field.type_ref.name,
            required: field.required,
            named: field.is_named(),
            flattened: field.flattened,
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

pub(crate) fn resolve_field_wire_contract(facts: FieldWireFacts<'_>) -> FieldWireContract {
    let declared_nullable = matches!(
        facts.type_name,
        "std::option::Option" | "reflectapi::Option"
    );
    FieldWireContract {
        key: if facts.flattened {
            KeyPresence::Flattened {
                optional: declared_nullable,
            }
        } else if facts.required {
            KeyPresence::Required
        } else {
            KeyPresence::Optional
        },
        declared_nullable,
        named: facts.named,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(type_name: &str, required: bool) -> FieldWireFacts<'_> {
        FieldWireFacts {
            type_name,
            required,
            named: true,
            flattened: false,
        }
    }

    #[test]
    fn required_is_authoritative_independently_of_declared_nullability() {
        for type_name in ["u8", "std::option::Option", "reflectapi::Option"] {
            for required in [true, false] {
                let contract = resolve_field_wire_contract(facts(type_name, required));
                assert_eq!(contract.key == KeyPresence::Required, required);
                assert_eq!(contract.declared_nullable, type_name != "u8");
            }
        }
    }

    #[test]
    fn old_or_manual_required_nullable_schema_stays_required() {
        let mut field = Field::new("value".into(), "std::option::Option".into());
        field.required = true;
        assert_eq!(
            resolve_field_wire_contract(FieldWireFacts::of(&field)).key,
            KeyPresence::Required
        );
    }

    #[test]
    fn only_an_optional_nullable_named_key_preserves_absence() {
        let contract = |type_name, required, named| {
            resolve_field_wire_contract(FieldWireFacts {
                named,
                ..facts(type_name, required)
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
            // `required` describes a key, which a flattened field doesn't have.
            for required in [true, false] {
                let contract = resolve_field_wire_contract(FieldWireFacts {
                    flattened: true,
                    ..facts(type_name, required)
                });
                let optional = type_name != "Inner";
                assert_eq!(contract.key, KeyPresence::Flattened { optional });
                assert_eq!(contract.flattened_group_required(), !optional);
                assert!(!contract.preserves_absence());
            }
        }
    }
}
