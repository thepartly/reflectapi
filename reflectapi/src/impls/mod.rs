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
    feature = "codegen",
    feature = "uuid",
    feature = "chrono",
    feature = "url",
    feature = "rust_decimal"
))]
mod tests {
    /// The formats are keyed by the types' names, so a renamed impl would lose
    /// its format without this.
    #[test]
    fn every_formatted_impl_is_named_as_the_format_table_knows_it() {
        fn check<T: crate::Output>(format: &str) {
            let mut typespace = crate::Typespace::new();
            let reference = T::reflectapi_output_type(&mut typespace);
            let ty = typespace.get_type(&reference.name).unwrap();
            let primitive = ty.as_primitive().unwrap();
            assert_eq!(
                crate::contract::format::string_format(primitive),
                Some(format),
                "{}",
                primitive.name()
            );
        }
        check::<uuid::Uuid>("uuid");
        check::<chrono::DateTime<chrono::Utc>>("date-time");
        check::<chrono::NaiveDateTime>("date-time-local");
        check::<chrono::NaiveDate>("date");
        check::<chrono::NaiveTime>("time-local");
        check::<url::Url>("uri");
        check::<rust_decimal::Decimal>("decimal");
    }
}
