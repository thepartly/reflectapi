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
//! contract is resolved against the schema being rendered.

use reflectapi_schema::{Field, Schema, TypeReference};

use super::nullability::accepts_null;

/// Which way the type holding a field crosses the wire. A custom codec only
/// matters on its own side: the server reads requests with
/// `deserialize_with` and writes responses with `serialize_with`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Request,
    Response,
    Both,
}

impl Direction {
    /// The direction of a type in a consolidated schema. A name in neither
    /// typespace, such as a class synthesized by a backend, is `Both`.
    pub fn of(schema: &Schema, type_name: &str) -> Self {
        match (
            schema.is_input_type(type_name),
            schema.is_output_type(type_name),
        ) {
            (true, false) => Direction::Request,
            (false, true) => Direction::Response,
            _ => Direction::Both,
        }
    }

    /// Whether the field has a custom codec that may read or write `null` for
    /// any type in this direction.
    pub(crate) fn custom_codec(self, serialize_with: bool, deserialize_with: bool) -> bool {
        match self {
            Direction::Request => deserialize_with,
            Direction::Response => serialize_with,
            Direction::Both => serialize_with || deserialize_with,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldWireContract {
    pub key: KeyPresence,
    // The declared Rust type admits null. Custom codecs may change its wire
    // representation; use a reflected type override for such codecs.
    pub declared_nullable: bool,
    /// `null` is a valid value in the field's direction. Unlike
    /// `declared_nullable`, this follows the type's definition: a unit struct
    /// or a newtype of an `Option` reads null, and so does a field with a
    /// custom codec on that side.
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

/// `schema` must be consolidated (`Schema::consolidate_types`), as every
/// generator does before rendering. Then a type name has one definition: a
/// request and a response type that share a name but differ are renamed to
/// `input::` and `output::` names, and so are the references to them.
pub(crate) fn resolve_field_wire_contract(
    facts: FieldWireFacts<'_>,
    schema: &Schema,
    direction: Direction,
) -> FieldWireContract {
    FieldWireContract {
        key: key_presence(facts),
        declared_nullable: is_option(facts.type_ref),
        value_accepts_null: direction.custom_codec(facts.serialize_with, facts.deserialize_with)
            || accepts_null(facts.type_ref, &|name| schema.get_type(name), direction),
        named: facts.named,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(facts: FieldWireFacts<'_>) -> FieldWireContract {
        resolve_field_wire_contract(facts, &Schema::new(), Direction::Both)
    }

    fn facts(type_ref: &TypeReference, required: bool) -> FieldWireFacts<'_> {
        FieldWireFacts {
            type_ref,
            required,
            named: true,
            flattened: false,
            serialize_with: false,
            deserialize_with: false,
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
    fn a_custom_codec_may_read_or_write_null_on_its_own_side() {
        let type_ref = TypeReference::from("u8");
        let accepts_null = |serialize_with, deserialize_with, direction| {
            resolve_field_wire_contract(
                FieldWireFacts {
                    serialize_with,
                    deserialize_with,
                    ..facts(&type_ref, false)
                },
                &Schema::new(),
                direction,
            )
            .value_accepts_null
        };
        assert!(accepts_null(false, true, Direction::Request));
        assert!(!accepts_null(true, false, Direction::Request));
        assert!(accepts_null(true, false, Direction::Response));
        assert!(!accepts_null(false, true, Direction::Response));
        assert!(accepts_null(true, false, Direction::Both));
        assert!(accepts_null(false, true, Direction::Both));
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

    #[test]
    fn a_request_and_a_response_type_sharing_a_name_resolve_separately() {
        let mut schema: Schema = serde_json::from_value(serde_json::json!({
            "name": "directions",
            "input_types": {"types": [
                {"kind": "struct", "name": "Payload", "fields": "none"},
                {"kind": "struct", "name": "Request", "fields": {"named": [
                    {"name": "payload", "type": {"name": "Payload"}}
                ]}}
            ]},
            "output_types": {"types": [
                {"kind": "primitive", "name": "u32"},
                {"kind": "struct", "name": "Payload", "fields": {"named": [
                    {"name": "a", "type": {"name": "u32"}, "required": true}
                ]}},
                {"kind": "struct", "name": "Response", "fields": {"named": [
                    {"name": "payload", "type": {"name": "Payload"}}
                ]}}
            ]}
        }))
        .unwrap();
        schema.consolidate_types();
        let payload_accepts_null = |owner: &str| {
            let owner = schema.get_type(owner).unwrap().as_struct().unwrap();
            let field = owner.fields.iter().next().unwrap();
            resolve_field_wire_contract(FieldWireFacts::of(field), &schema, Direction::Both)
                .value_accepts_null
        };
        // The request's unit struct reads null; the response's struct doesn't.
        assert!(payload_accepts_null("Request"));
        assert!(!payload_accepts_null("Response"));
    }
}
