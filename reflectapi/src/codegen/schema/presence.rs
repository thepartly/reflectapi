//! Rendering facts for field presence and declared nullability.
//!
//! Reflection resolves Serde's directional missing-key behavior into
//! `Field.required` before input/output types are consolidated. Renderers must
//! preserve that decision, including overrides made by schema transforms and
//! required nullable fields in manually constructed or older schemas.

use reflectapi_schema::Field;

#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldWireFacts<'a> {
    pub type_name: &'a str,
    pub required: bool,
}

impl<'a> FieldWireFacts<'a> {
    pub fn of(field: &'a Field) -> Self {
        Self {
            type_name: &field.type_ref.name,
            required: field.required,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldWireContract {
    pub key: KeyPresence,
    // The declared Rust type admits null. Custom codecs may change its wire
    // representation; use a reflected type override for such codecs.
    pub declared_nullable: bool,
}

pub(crate) fn resolve_field_wire_contract(facts: FieldWireFacts<'_>) -> FieldWireContract {
    FieldWireContract {
        key: if facts.required {
            KeyPresence::Required
        } else {
            KeyPresence::Optional
        },
        declared_nullable: matches!(
            facts.type_name,
            "std::option::Option" | "reflectapi::Option"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_is_authoritative_independently_of_declared_nullability() {
        for type_name in ["u8", "std::option::Option", "reflectapi::Option"] {
            for required in [true, false] {
                let contract = resolve_field_wire_contract(FieldWireFacts {
                    type_name,
                    required,
                });
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
}
