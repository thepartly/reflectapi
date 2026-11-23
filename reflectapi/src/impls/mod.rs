#[cfg(feature = "uuid")]
mod uuid;

#[cfg(feature = "chrono")]
mod chrono;

#[cfg(feature = "chrono-tz")]
mod chrono_tz;

#[cfg(feature = "url")]
mod url;

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "rust_decimal")]
mod rust_decimal;

#[cfg(feature = "indexmap")]
mod indexmap;

/// `wire_format` stands in for these impls' formats on schemas written before
/// primitives carried one, so it must give the same formats they set.
#[cfg(all(
    test,
    feature = "uuid",
    feature = "chrono",
    feature = "url",
    feature = "rust_decimal"
))]
mod tests {
    #[test]
    fn older_schemas_get_the_formats_the_impls_set() {
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
            let mut older = primitive.clone();
            older.format = None;
            assert_eq!(
                reflectapi_schema::wire::format::wire_format(&older),
                primitive.format()
            );
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
