//! What a primitive is on the wire beyond its JSON type, such as a UUID, a date
//! or a URI, in each serialization format. The type's name says what it is,
//! so the forms are a table keyed by name, and every generator reads it here.

use crate::{Primitive, SerializationMode};

/// How a primitive is written in one serialization format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireForm {
    /// A string with a format: a JSON Schema format such as `uuid` or
    /// `date-time`, or a type's own such as `decimal`.
    String { format: &'static str },
    /// Raw bytes, `length` of them.
    Bytes { length: usize },
}

/// How the primitive named `name` is written in `mode`, if it's one reflectapi
/// knows. A type's `Serialize` may write it differently in each format: a UUID
/// is a string in JSON and 16 bytes in MessagePack, since serde's `uuid` impl
/// checks `is_human_readable()`.
pub fn wire_form(name: &str, mode: SerializationMode) -> Option<WireForm> {
    let format = match name {
        "uuid::Uuid" => match mode {
            SerializationMode::Json => "uuid",
            SerializationMode::Msgpack => return Some(WireForm::Bytes { length: 16 }),
        },
        "chrono::DateTime" => "date-time",
        // RFC 3339's `date-time` and `time` require an offset; serde writes
        // chrono's naive types without one.
        "chrono::NaiveDateTime" => "date-time-local",
        "chrono::NaiveDate" => "date",
        "chrono::NaiveTime" => "time-local",
        "url::Url" => "uri",
        "rust_decimal::Decimal" => "decimal",
        _ => return None,
    };
    Some(WireForm::String { format })
}

/// The format of the string a primitive is written as in JSON, if it's one
/// reflectapi knows.
pub fn string_format(primitive: &Primitive) -> Option<&'static str> {
    match wire_form(primitive.name(), SerializationMode::Json)? {
        WireForm::String { format } => Some(format),
        WireForm::Bytes { .. } => None,
    }
}

/// How a Rust integer type is written: its OpenAPI format, and the bounds a
/// JSON number must keep within, where they fit in an `i64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegerWidth {
    /// The OpenAPI format registry's name: `int8` … `int64`, `uint8` … `uint64`.
    /// `None` for `isize` and `usize`, whose width is the server's.
    pub format: Option<&'static str>,
    pub minimum: Option<i64>,
    pub maximum: Option<i64>,
}

/// The width of a primitive integer type, by name. `i128` and `u128` have no
/// registered format. `isize` and `usize` are as wide as the server's
/// pointers, 32 bits on wasm32, which the schema doesn't record, so they get
/// no format and `usize` only its minimum.
pub fn integer_width(name: &str) -> Option<IntegerWidth> {
    let (format, minimum, maximum) = match name {
        "i8" => (Some("int8"), Some(i8::MIN.into()), Some(i8::MAX.into())),
        "i16" => (Some("int16"), Some(i16::MIN.into()), Some(i16::MAX.into())),
        "i32" => (Some("int32"), Some(i32::MIN.into()), Some(i32::MAX.into())),
        "i64" => (Some("int64"), None, None),
        "u8" => (Some("uint8"), Some(0), Some(u8::MAX.into())),
        "u16" => (Some("uint16"), Some(0), Some(u16::MAX.into())),
        "u32" => (Some("uint32"), Some(0), Some(u32::MAX.into())),
        "u64" => (Some("uint64"), Some(0), None),
        "usize" => (None, Some(0), None),
        _ => return None,
    };
    Some(IntegerWidth {
        format,
        minimum,
        maximum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primitive(name: &str) -> Primitive {
        Primitive::new(
            name.into(),
            String::new(),
            vec![],
            Some("std::string::String".into()),
        )
    }

    #[test]
    fn a_primitive_reflectapi_knows_has_its_format_by_name() {
        assert_eq!(string_format(&primitive("uuid::Uuid")), Some("uuid"));
        assert_eq!(string_format(&primitive("chrono::NaiveDate")), Some("date"));
        assert_eq!(string_format(&primitive("my::Sku")), None);
    }

    #[test]
    fn a_uuid_is_a_string_in_json_and_bytes_in_messagepack() {
        assert_eq!(
            wire_form("uuid::Uuid", SerializationMode::Json),
            Some(WireForm::String { format: "uuid" })
        );
        assert_eq!(
            wire_form("uuid::Uuid", SerializationMode::Msgpack),
            Some(WireForm::Bytes { length: 16 })
        );
        // chrono doesn't check `is_human_readable()`: a string in both.
        assert_eq!(
            wire_form("chrono::DateTime", SerializationMode::Msgpack),
            Some(WireForm::String {
                format: "date-time"
            })
        );
    }

    #[test]
    fn integers_carry_their_registered_format_and_bounds() {
        let u8 = integer_width("u8").unwrap();
        assert_eq!(
            (u8.format, u8.minimum, u8.maximum),
            (Some("uint8"), Some(0), Some(255))
        );
        let u64 = integer_width("u64").unwrap();
        assert_eq!(
            (u64.format, u64.minimum, u64.maximum),
            (Some("uint64"), Some(0), None)
        );
        let usize = integer_width("usize").unwrap();
        assert_eq!((usize.format, usize.minimum), (None, Some(0)));
        assert_eq!(integer_width("isize"), None);
        assert_eq!(integer_width("i64").unwrap().minimum, None);
        assert_eq!(integer_width("u128"), None);
    }
}
