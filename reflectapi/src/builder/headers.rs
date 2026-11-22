//! Reads a route's request headers type from the request's headers.
//!
//! A header's value is text. Each one is given to serde as a string, and
//! parsed when serde asks for a number, a `bool` or a `char`, so a headers
//! type can have such fields as well as strings, unit-variant enums and types
//! read from strings (`Uuid`, `DateTime` and the like).
//!
//! Inside a `#[serde(flatten)]` field values are only strings: serde buffers
//! a flattened field's values before it knows their types.

use std::fmt;

use serde::de::{self, value::BorrowedStrDeserializer, DeserializeSeed, MapAccess, Visitor};
use serde::forward_to_deserialize_any;

/// Why the headers didn't make the route's headers type.
#[derive(Debug)]
pub(crate) struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

/// Deserializes `H` from header names and values. Names are lowercase, as
/// `http::HeaderName` gives them.
pub(crate) fn from_headers<H: de::DeserializeOwned>(
    headers: &[(String, String)],
) -> Result<H, Error> {
    H::deserialize(Headers(headers))
}

/// The headers, as a map from name to value.
struct Headers<'a>(&'a [(String, String)]);

impl<'de> de::Deserializer<'de> for Headers<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_map(Entries {
            entries: self.0.iter(),
            value: None,
        })
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct Entries<'a> {
    entries: std::slice::Iter<'a, (String, String)>,
    value: Option<&'a str>,
}

impl<'de> MapAccess<'de> for Entries<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match self.entries.next() {
            Some((name, value)) => {
                self.value = Some(value);
                seed.deserialize(BorrowedStrDeserializer::new(name))
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        let value = self
            .value
            .take()
            .ok_or_else(|| Error("a header value without its name".into()))?;
        seed.deserialize(Value(value))
    }
}

/// One header's value: a string, parsed on request.
struct Value<'a>(&'a str);

impl Value<'_> {
    fn parse<T: std::str::FromStr>(&self, expected: &dyn de::Expected) -> Result<T, Error> {
        self.0
            .trim()
            .parse()
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(self.0), expected))
    }
}

macro_rules! parse_into {
    ($($deserialize:ident => $visit:ident,)*) => {
        $(
            fn $deserialize<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
                let value = self.parse(&visitor)?;
                visitor.$visit(value)
            }
        )*
    };
}

impl<'de> de::Deserializer<'de> for Value<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_borrowed_str(self.0)
    }

    parse_into! {
        deserialize_bool => visit_bool,
        deserialize_i8 => visit_i8,
        deserialize_i16 => visit_i16,
        deserialize_i32 => visit_i32,
        deserialize_i64 => visit_i64,
        deserialize_i128 => visit_i128,
        deserialize_u8 => visit_u8,
        deserialize_u16 => visit_u16,
        deserialize_u32 => visit_u32,
        deserialize_u64 => visit_u64,
        deserialize_u128 => visit_u128,
        deserialize_f32 => visit_f32,
        deserialize_f64 => visit_f64,
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let mut chars = self.0.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(de::Error::invalid_value(
                de::Unexpected::Str(self.0),
                &visitor,
            )),
        }
    }

    /// A header that is present is `Some`; a missing one never gets here.
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    /// A unit variant, named by the header's value.
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_enum(BorrowedStrDeserializer::new(self.0))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        str string bytes byte_buf unit unit_struct seq tuple tuple_struct map
        struct identifier
    }
}

#[cfg(test)]
mod tests {
    use super::from_headers;

    fn headers(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "kebab-case")]
    enum Mode {
        Fast,
        Safe,
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Common {
        #[serde(rename = "x-request-id")]
        request_id: Option<String>,
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct All {
        authorization: String,
        #[serde(rename = "x-limit")]
        limit: u32,
        #[serde(rename = "x-offset")]
        offset: Option<i64>,
        #[serde(rename = "x-dry-run")]
        dry_run: Option<bool>,
        #[serde(rename = "x-ratio")]
        ratio: Option<f64>,
        #[serde(rename = "x-mode")]
        mode: Option<Mode>,
        #[serde(rename = "x-grade")]
        grade: Option<char>,
        #[serde(flatten)]
        common: Common,
    }

    #[test]
    fn reads_strings_numbers_bools_enums_and_chars() {
        let read: All = from_headers(&headers(&[
            ("authorization", "secret"),
            ("x-limit", "25"),
            ("x-offset", "-3"),
            ("x-dry-run", "true"),
            ("x-ratio", "0.5"),
            ("x-mode", "safe"),
            ("x-grade", "A"),
            ("x-request-id", "42"),
            ("cookie", "ignored"),
        ]))
        .unwrap();
        assert_eq!(
            read,
            All {
                authorization: "secret".into(),
                limit: 25,
                offset: Some(-3),
                dry_run: Some(true),
                ratio: Some(0.5),
                mode: Some(Mode::Safe),
                grade: Some('A'),
                // A number-looking value stays a string where a string is asked for.
                common: Common {
                    request_id: Some("42".into())
                },
            }
        );
    }

    #[test]
    fn missing_optional_headers_are_none() {
        let read: All =
            from_headers(&headers(&[("authorization", "secret"), ("x-limit", "1")])).unwrap();
        assert_eq!(read.offset, None);
        assert_eq!(read.dry_run, None);
        assert_eq!(read.common, Common { request_id: None });
    }

    #[test]
    fn a_missing_required_header_is_named() {
        let err = from_headers::<All>(&headers(&[("x-limit", "1")])).unwrap_err();
        assert_eq!(err.to_string(), "missing field `authorization`");
    }

    #[test]
    fn a_value_that_doesnt_parse_says_what_was_expected() {
        let err = from_headers::<All>(&headers(&[("authorization", "s"), ("x-limit", "lots")]))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid value: string \"lots\", expected u32"
        );

        let err = from_headers::<All>(&headers(&[
            ("authorization", "s"),
            ("x-limit", "1"),
            ("x-mode", "slow"),
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("unknown variant `slow`"), "{err}");
    }

    #[test]
    fn numbers_inside_a_flattened_field_are_strings() {
        #[derive(serde::Deserialize, Debug)]
        struct Paging {
            #[serde(rename = "x-page")]
            #[allow(dead_code)]
            page: u32,
        }
        #[derive(serde::Deserialize, Debug)]
        struct Outer {
            #[serde(flatten)]
            #[allow(dead_code)]
            paging: Paging,
        }
        let err = from_headers::<Outer>(&headers(&[("x-page", "2")])).unwrap_err();
        assert!(
            err.to_string().contains("invalid type: string \"2\""),
            "{err}"
        );
    }
}
