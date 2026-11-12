//! Whether a field's key may be absent from the object that holds it.
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

use reflectapi_schema::{Field, TypeReference};

#[derive(Debug, Clone, Copy)]
pub(crate) struct FieldWireFacts<'a> {
    pub type_ref: &'a TypeReference,
    pub required: bool,
    pub named: bool,
    pub flattened: bool,
    pub serialize_with: bool,
    pub deserialize_with: bool,
}

impl<'a> FieldWireFacts<'a> {
    pub fn of(field: &'a Field) -> Self {
        Self {
            type_ref: &field.type_ref,
            required: field.required,
            named: field.is_named(),
            flattened: field.flattened,
            serialize_with: field.serialize_with,
            deserialize_with: field.deserialize_with,
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

pub(crate) fn is_option(type_ref: &TypeReference) -> bool {
    matches!(
        type_ref.name.as_str(),
        "std::option::Option" | "reflectapi::Option"
    )
}

/// The key half of a field's wire contract, which needs no typespace.
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
