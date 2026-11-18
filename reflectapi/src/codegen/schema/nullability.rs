//! Whether serde reads JSON `null` into a type.
//!
//! A field that may be omitted but whose type rejects `null` has two wire
//! states, absent or a value. Clients that represent "absent" with a null
//! sentinel must not send that sentinel, so backends ask here.

use std::collections::BTreeSet;

use reflectapi_schema::{Field, Representation, Type, TypeReference};

use super::presence::Direction;
use crate::wire::layout::Lookup;
use crate::wire::shape::Shape;

/// Conservative: anything not known to reject `null`, such as a generic
/// parameter or a type the typespace doesn't define, is assumed to accept it,
/// so a client never refuses a value the server would read.
pub(crate) fn accepts_null(
    type_ref: &TypeReference,
    lookup: Lookup<'_, '_>,
    direction: Direction,
) -> bool {
    Resolver { lookup, direction }.reads_null(type_ref, &mut BTreeSet::new())
}

/// Wrappers that serde reads and writes as the `T` they hold.
fn unwrap_pointers(type_ref: &TypeReference) -> TypeReference {
    let mut current = type_ref.clone();
    let mut seen = BTreeSet::new();
    while matches!(
        current.name.as_str(),
        "std::boxed::Box"
            | "std::rc::Rc"
            | "std::sync::Arc"
            | "std::borrow::Cow"
            | "std::cell::Cell"
            | "std::cell::RefCell"
            | "std::sync::Mutex"
            | "std::sync::RwLock"
    ) && seen.insert(current.clone())
    {
        match current.arguments.first() {
            Some(inner) => current = inner.clone(),
            None => break,
        }
    }
    current
}

/// Types whose serde representation never includes `null`, known by name
/// because their definition doesn't say so: the schema shows
/// `reflectapi::Empty` (`struct Empty {}`) and `reflectapi::Infallible` (an
/// enum without variants) as unit structs.
fn primitive_rejects_null(name: &str) -> bool {
    matches!(
        name,
        "reflectapi::Empty"
            | "reflectapi::Infallible"
            | "i8"
            | "i16"
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
            | "url::Url"
            | "rust_decimal::Decimal"
            | "chrono::DateTime"
            | "chrono::NaiveDateTime"
            | "chrono::NaiveDate"
            | "chrono::NaiveTime"
            | "chrono_tz::Tz"
            | "std::time::Duration"
            | "std::path::PathBuf"
            | "std::path::Path"
            | "std::net::IpAddr"
            | "std::net::Ipv4Addr"
            | "std::net::Ipv6Addr"
            | "std::result::Result"
            | "std::num::NonZeroU8"
            | "std::num::NonZeroU16"
            | "std::num::NonZeroU32"
            | "std::num::NonZeroU64"
            | "std::num::NonZeroU128"
            | "std::array::Array"
    ) || name
        .strip_prefix("std::tuple::Tuple")
        .and_then(|arity| arity.parse::<u8>().ok())
        .is_some_and(|arity| arity > 0)
}

struct Resolver<'a, 'l> {
    lookup: Lookup<'a, 'l>,
    direction: Direction,
}

impl Resolver<'_, '_> {
    /// A wrapped field with a custom codec on this side may read null
    /// whatever its type.
    fn field_reads_null(
        &self,
        field: &Field,
        type_ref: &TypeReference,
        visiting: &mut BTreeSet<TypeReference>,
    ) -> bool {
        self.direction
            .custom_codec(field.serialize_with, field.deserialize_with)
            || self.reads_null(type_ref, visiting)
    }

    fn reads_null(&self, type_ref: &TypeReference, visiting: &mut BTreeSet<TypeReference>) -> bool {
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
        let result = match (self.lookup)(&type_ref.name) {
            None => true,
            // A primitive's fallback describes its representation, not what its
            // decoder accepts (`Weak<T>` falls back to `T` but reads null).
            Some(Type::Primitive(_)) => true,
            Some(Type::Struct(s)) => {
                let bind = |inner: &TypeReference| substitute(inner, s.parameters(), &type_ref);
                if s.transparent {
                    s.fields.iter().next().is_none_or(|field| {
                        self.field_reads_null(field, &bind(&field.type_ref), visiting)
                    })
                } else {
                    match Shape::of(&s.fields) {
                        Shape::Unit => true,
                        Shape::Newtype(field) => {
                            self.field_reads_null(field, &bind(&field.type_ref), visiting)
                        }
                        // A backend that strips `PhantomData` fields leaves
                        // `struct Marker<T>(PhantomData<T>)`, which reads null, as
                        // an empty tuple struct.
                        Shape::Tuple([]) => true,
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
                        Shape::Unit | Shape::Tuple([]) => true,
                        Shape::Newtype(field) => self.field_reads_null(
                            field,
                            &substitute(&field.type_ref, e.parameters(), &type_ref),
                            visiting,
                        ),
                        Shape::Tuple(_) | Shape::Struct(_) => false,
                    })
            }
        };
        visiting.remove(&type_ref);
        result
    }
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

    fn schema() -> reflectapi_schema::Schema {
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
                {"kind": "struct", "name": "Lenient", "fields": {"unnamed": [
                    {"name": "0", "type": {"name": "u32"}, "deserialize_with": true}
                ]}},
                {"kind": "struct", "name": "Sorted", "fields": {"unnamed": [
                    {"name": "0", "type": {"name": "u32"}, "serialize_with": true}
                ]}},
                {"kind": "struct", "name": "StrippedMarker", "fields": {"unnamed": []}},
                {"kind": "struct", "name": "reflectapi::Empty", "fields": "none"},
                {"kind": "enum", "name": "UntaggedValue", "representation": "none",
                 "variants": [{"name": "B", "fields": {"unnamed": [{"name": "0", "type": {"name": "u32"}}]}}]}
            ]}
        }))
        .unwrap()
    }

    fn accepts(type_ref: serde_json::Value) -> bool {
        let type_ref: TypeReference = serde_json::from_value(type_ref).unwrap();
        let schema = schema();
        accepts_null(
            &type_ref,
            &|name| schema.input_types.get_type(name),
            Direction::Both,
        )
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
            "reflectapi::Empty",
        ] {
            assert!(!accepts(serde_json::json!({"name": name})), "{name}");
        }
    }

    #[test]
    fn units_untagged_units_and_wrapped_options_accept_null() {
        for name in [
            "Unit",
            "MaybeId",
            "Lenient",
            "StrippedMarker",
            "UntaggedUnit",
            "Loop",
            "TaggedWithUntaggedUnit",
        ] {
            assert!(accepts(serde_json::json!({"name": name})), "{name}");
        }
    }

    #[test]
    fn transparent_and_pointer_wrappers_follow_what_they_wrap() {
        let wrap = |argument: serde_json::Value| {
            accepts(serde_json::json!({"name": "Wrap", "arguments": [argument]}))
        };
        assert!(!wrap(serde_json::json!({"name": "u32"})));
        assert!(wrap(
            serde_json::json!({"name": "std::option::Option", "arguments": [{"name": "u32"}]})
        ));
        assert!(wrap(serde_json::json!({"name": "T"})));
        for pointer in [
            "std::boxed::Box",
            "std::borrow::Cow",
            "std::cell::RefCell",
            "std::sync::Mutex",
        ] {
            assert!(
                !accepts(serde_json::json!({"name": pointer, "arguments": [{"name": "Named"}]})),
                "{pointer}"
            );
            assert!(
                accepts(serde_json::json!({"name": pointer, "arguments": [{"name": "Unit"}]})),
                "{pointer}"
            );
        }
    }

    #[test]
    fn a_newtype_follows_the_type_it_wraps() {
        let new_of = |argument: serde_json::Value| {
            accepts(serde_json::json!({"name": "NewOf", "arguments": [argument]}))
        };
        assert!(!new_of(serde_json::json!({"name": "Named"})));
        assert!(!new_of(serde_json::json!({"name": "u32"})));
        assert!(new_of(serde_json::json!({"name": "serde_json::Value"})));
        assert!(new_of(serde_json::json!({"name": "std::tuple::Tuple0"})));
        assert!(new_of(
            serde_json::json!({"name": "std::sync::Weak", "arguments": [{"name": "Named"}]})
        ));
    }

    #[test]
    fn primitives_answer_for_themselves() {
        for name in [
            "u32",
            "std::string::String",
            "chrono::DateTime",
            "chrono::NaiveTime",
            "std::array::Array",
            "std::tuple::Tuple2",
        ] {
            assert!(!accepts(serde_json::json!({"name": name})), "{name}");
        }
        for name in [
            "std::option::Option",
            "std::tuple::Tuple0",
            "serde_json::Value",
            "T",
            "unknown::Type",
        ] {
            assert!(accepts(serde_json::json!({"name": name})), "{name}");
        }
    }

    #[test]
    fn a_wrapped_codec_counts_only_on_its_own_side() {
        let schema = schema();
        let lookup = |name: &str| schema.input_types.get_type(name);
        let accepts =
            |name: &str, direction| accepts_null(&TypeReference::from(name), &lookup, direction);
        assert!(accepts("Lenient", Direction::Request));
        assert!(!accepts("Lenient", Direction::Response));
        assert!(accepts("Sorted", Direction::Response));
        assert!(!accepts("Sorted", Direction::Request));
        assert!(accepts("Sorted", Direction::Both));
    }
}
