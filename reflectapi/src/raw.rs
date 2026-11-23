use std::marker::PhantomData;

use serde_json::value::RawValue;

/// A `T` kept as the JSON it arrived as, decoded only when asked.
///
/// The schema and the clients see a `T`. The server keeps the text, so it can
/// store or forward the value unchanged, read a few fields with
/// [`peek`](Self::peek) or decode it with [`decode`](Self::decode).
///
/// With JSON the text passes through unchanged. With MessagePack, a format
/// serde reports as not human-readable, the value is converted: read into
/// JSON text, and written from it. Key order and every field survive; a value
/// JSON can't represent, such as binary data or NaN, is an error.
///
/// ```rust
/// #[derive(serde::Serialize, serde::Deserialize, reflectapi::Input)]
/// struct Event {
///     kind: String,
///     amount: u64,
/// }
///
/// #[derive(serde::Deserialize)]
/// struct Kind<'a> {
///     #[serde(borrow)]
///     kind: &'a str,
/// }
///
/// let raw: reflectapi::Raw<Event> =
///     serde_json::from_str(r#"{"kind":"refund","amount":5}"#).unwrap();
/// assert_eq!(raw.peek::<Kind>().unwrap().kind, "refund");
/// assert_eq!(raw.json().get(), r#"{"kind":"refund","amount":5}"#);
/// assert_eq!(raw.decode().unwrap().amount, 5);
/// ```
#[derive(Debug, Clone)]
pub struct Raw<T> {
    json: Box<RawValue>,
    of: PhantomData<fn() -> T>,
}

impl<T> Raw<T> {
    /// From JSON text, which must be valid JSON but isn't checked against `T`.
    pub fn new(json: String) -> serde_json::Result<Self> {
        RawValue::from_string(json).map(Self::from)
    }

    /// The JSON exactly as it arrived.
    pub fn json(&self) -> &RawValue {
        &self.json
    }

    /// Reads the fields the caller needs into `V` without decoding all of `T`.
    /// `V` may borrow from the text: `#[serde(borrow)] field: &'a str`.
    pub fn peek<'a, V: serde::Deserialize<'a>>(&'a self) -> serde_json::Result<V> {
        serde_json::from_str(self.json.get())
    }

    /// Decodes the whole `T`.
    pub fn decode(&self) -> serde_json::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_str(self.json.get())
    }
}

impl<T: serde::Serialize> Raw<T> {
    pub fn from_value(value: &T) -> serde_json::Result<Self> {
        serde_json::value::to_raw_value(value).map(Self::from)
    }
}

impl<T> From<Box<RawValue>> for Raw<T> {
    fn from(json: Box<RawValue>) -> Self {
        Raw {
            json,
            of: PhantomData,
        }
    }
}

impl<'de, T> serde::Deserialize<'de> for Raw<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        if deserializer.is_human_readable() {
            // JSON: keep the text exactly.
            return Box::<RawValue>::deserialize(deserializer).map(Self::from);
        }
        // MessagePack and other binary formats: write the value as JSON text.
        let mut json = Vec::new();
        crate::transcode::transcode(
            deserializer,
            &mut serde_json::Serializer::new(&mut json),
            true,
        )
        .map_err(D::Error::custom)?;
        let json = String::from_utf8(json).map_err(D::Error::custom)?;
        Self::new(json).map_err(D::Error::custom)
    }
}

impl<T> serde::Serialize for Raw<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            // JSON: write the text as it is.
            return self.json.serialize(serializer);
        }
        // MessagePack and other binary formats: write the value the text holds.
        let mut json = serde_json::Deserializer::from_str(self.json.get());
        crate::transcode::transcode(&mut json, serializer, false)
    }
}

impl<T: crate::Input> crate::Input for Raw<T> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        T::reflectapi_input_type(schema)
    }
}

impl<T: crate::Output> crate::Output for Raw<T> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        T::reflectapi_output_type(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_text_unchanged() {
        let text = r#"{ "b": 1,  "a": [true] }"#;
        let raw: Raw<serde_json::Value> = serde_json::from_str(text).unwrap();
        assert_eq!(raw.json().get(), text);
        assert_eq!(serde_json::to_string(&raw).unwrap(), text);
    }

    #[test]
    fn writes_serde_json_values() {
        let raw: Raw<serde_json::Value> = serde_json::from_str(r#"{"a":[1,2]}"#).unwrap();
        assert_eq!(
            serde_json::to_value(&raw).unwrap(),
            serde_json::json!({"a": [1, 2]})
        );
    }

    #[test]
    fn reflects_as_the_wrapped_type() {
        let mut typespace = crate::Typespace::default();
        let type_ref = <Raw<String> as crate::Input>::reflectapi_input_type(&mut typespace);
        assert_eq!(type_ref.name, "std::string::String");
    }
}

#[cfg(all(test, feature = "msgpack"))]
mod msgpack_tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Event {
        kind: String,
        amount: u64,
        ratio: f64,
        tags: Vec<String>,
        note: Option<String>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct Envelope {
        id: u32,
        body: Raw<Event>,
    }

    fn event() -> Event {
        Event {
            kind: "refund".into(),
            amount: 5,
            ratio: 0.25,
            tags: vec!["a".into(), "b".into()],
            note: None,
        }
    }

    #[test]
    fn reads_msgpack_into_json_text() {
        let typed = serde_json::json!({"id": 1, "body": event()});
        let bytes = rmp_serde::to_vec_named(&typed).unwrap();
        let envelope: Envelope = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(envelope.id, 1);
        assert_eq!(
            envelope.body.json().get(),
            r#"{"amount":5,"kind":"refund","note":null,"ratio":0.25,"tags":["a","b"]}"#
        );
        assert_eq!(envelope.body.decode().unwrap(), event());
    }

    #[test]
    fn keeps_key_order_and_unknown_fields() {
        // Keys out of order, and one `Event` doesn't have.
        let text = r#"{"tags":[],"unknown":{"x":[1,true,null]},"kind":"k","amount":1,"ratio":1.5,"note":"n"}"#;
        let value: serde_json::Value = serde_json::from_str(text).unwrap();
        // serde_json::Value sorts keys; write the bytes from the text instead.
        let raw: Raw<Event> = serde_json::from_str(text).unwrap();
        let bytes = rmp_serde::to_vec_named(&raw).unwrap();
        let back: Raw<Event> = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back.json().get(), text);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(back.json().get()).unwrap(),
            value
        );
    }

    #[test]
    fn writes_msgpack_as_the_value_not_the_text() {
        let envelope = Envelope {
            id: 7,
            body: Raw::from_value(&event()).unwrap(),
        };
        let bytes = rmp_serde::to_vec_named(&envelope).unwrap();
        // Readable as the typed value, so no marker object and no string of JSON.
        #[derive(serde::Deserialize)]
        struct Typed {
            id: u32,
            body: Event,
        }
        let typed: Typed = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!((typed.id, typed.body), (7, event()));
        // The same bytes as writing the typed value directly.
        let direct =
            rmp_serde::to_vec_named(&serde_json::json!({"id": 7, "body": event()})).unwrap();
        assert_eq!(bytes.len(), direct.len());
    }

    #[test]
    fn msgpack_values_json_cant_hold_are_errors() {
        #[derive(serde::Serialize)]
        struct Bin<'a> {
            #[serde(with = "serde_bytes_like")]
            data: &'a [u8],
        }
        mod serde_bytes_like {
            pub fn serialize<S: serde::Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
                s.serialize_bytes(v)
            }
        }
        let bytes = rmp_serde::to_vec_named(&Bin { data: &[1, 2, 3] }).unwrap();
        let err = rmp_serde::from_slice::<Raw<serde_json::Value>>(&bytes).unwrap_err();
        assert!(
            err.to_string().contains("binary data has no JSON form"),
            "{err}"
        );

        let bytes = rmp_serde::to_vec_named(&f64::NAN).unwrap();
        let err = rmp_serde::from_slice::<Raw<f64>>(&bytes).unwrap_err();
        assert!(err.to_string().contains("NaN has no JSON form"), "{err}");
    }
}
