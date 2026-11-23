//! What a primitive is on the wire beyond its JSON type: a UUID, a date, a
//! URI. Every generator reads it here, so a schema says it once.

use crate::Primitive;

/// The format of the string a primitive is written as: the schema's, or, for
/// a schema written before primitives carried one, the built-in type's.
pub fn wire_format(primitive: &Primitive) -> Option<&str> {
    primitive
        .format()
        .or_else(|| builtin_format(primitive.name()))
}

/// The formats reflectapi's own impls set, by type name.
fn builtin_format(name: &str) -> Option<&'static str> {
    Some(match name {
        "uuid::Uuid" => "uuid",
        "chrono::DateTime" => "date-time",
        // RFC 3339's `date-time` and `time` require an offset; serde writes
        // chrono's naive types without one.
        "chrono::NaiveDateTime" => "date-time-local",
        "chrono::NaiveDate" => "date",
        "chrono::NaiveTime" => "time-local",
        "url::Url" => "uri",
        "rust_decimal::Decimal" => "decimal",
        _ => return None,
    })
}

/// A primitive written as a string with a format: one that has a format and
/// either no fallback or a `String` one. A fallback to anything else says
/// how it's written instead.
pub fn string_format(primitive: &Primitive) -> Option<&str> {
    let format = wire_format(primitive)?;
    match primitive.fallback() {
        None => Some(format),
        Some(fallback) if fallback.name == "std::string::String" => Some(format),
        Some(_) => None,
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

    fn primitive(name: &str, fallback: Option<&str>) -> Primitive {
        Primitive::new(name.into(), String::new(), vec![], fallback.map(Into::into))
    }

    #[test]
    fn a_schema_written_before_formats_gets_the_built_in_ones() {
        assert_eq!(string_format(&primitive("uuid::Uuid", None)), Some("uuid"));
        assert_eq!(
            string_format(&primitive("chrono::NaiveDate", Some("std::string::String"))),
            Some("date")
        );
        assert_eq!(
            string_format(&primitive("my::Sku", Some("std::string::String"))),
            None
        );
    }

    #[test]
    fn a_type_can_give_its_own_format_but_not_to_a_non_string() {
        let sku = primitive("my::Sku", Some("std::string::String")).with_format("sku");
        assert_eq!(string_format(&sku), Some("sku"));
        let count = primitive("my::Count", Some("u64")).with_format("int64");
        assert_eq!(wire_format(&count), Some("int64"));
        assert_eq!(string_format(&count), None);
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
