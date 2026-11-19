//! What a primitive is on the wire beyond its JSON type: a UUID, a date, a
//! URI. Every generator reads it here, so a schema says it once.

use reflectapi_schema::Primitive;

/// The format of the string a primitive is written as: the schema's, or, for
/// a schema written before primitives carried one, the built-in type's.
pub(crate) fn wire_format(primitive: &Primitive) -> Option<&str> {
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
pub(crate) fn string_format(primitive: &Primitive) -> Option<&str> {
    let format = wire_format(primitive)?;
    match primitive.fallback() {
        None => Some(format),
        Some(fallback) if fallback.name == "std::string::String" => Some(format),
        Some(_) => None,
    }
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

    /// The table above stands in for the impls' formats on older schemas,
    /// so it must say the same as they do.
    #[cfg(all(
        feature = "uuid",
        feature = "chrono",
        feature = "url",
        feature = "rust_decimal"
    ))]
    #[test]
    fn the_built_in_impls_set_the_formats_the_table_gives() {
        fn check<T: crate::Output>() {
            let mut typespace = crate::Typespace::new();
            let reference = T::reflectapi_output_type(&mut typespace);
            let ty = typespace.get_type(&reference.name).unwrap();
            let primitive = ty.as_primitive().unwrap();
            assert!(
                primitive.format().is_some(),
                "{} sets no format",
                primitive.name()
            );
            assert_eq!(primitive.format(), builtin_format(primitive.name()));
        }
        check::<uuid::Uuid>();
        check::<chrono::DateTime<chrono::Utc>>();
        check::<chrono::NaiveDateTime>();
        check::<chrono::NaiveDate>();
        check::<chrono::NaiveTime>();
        check::<url::Url>();
        check::<rust_decimal::Decimal>();
    }
}
