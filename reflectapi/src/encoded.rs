use std::ops::Deref;
use std::str::FromStr;

use base64::Engine;

const BASE64: base64::engine::GeneralPurpose = base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A typed value encoded into a string, for clients to store and send back
/// without reading it, such as a page cursor.
///
/// The server works with the typed `T`. On the wire, and in the schema and the
/// clients, it is a `string`: `T` as JSON, then base64url without padding, so
/// it is safe in URLs and headers. Clients should treat the format as
/// unstable.
///
/// **Encoded, not secret.** Anyone can decode the string, read `T`, and send
/// back a changed one. Check what you decode as you would any other input,
/// and don't put anything in it the client mustn't see. A value that must be
/// tamper-proof needs signing, which this type doesn't do.
///
/// ```rust
/// use reflectapi::Encoded;
///
/// #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
/// struct Cursor {
///     after_id: u64,
/// }
///
/// let cursor = Encoded(Cursor { after_id: 42 });
/// let text = cursor.encode().unwrap();
/// assert_eq!(text, "eyJhZnRlcl9pZCI6NDJ9");
/// assert_eq!(Encoded::<Cursor>::decode(&text).unwrap().after_id, 42);
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Encoded<T>(pub T);

impl<T> Encoded<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: serde::Serialize> Encoded<T> {
    /// The string clients see.
    pub fn encode(&self) -> serde_json::Result<String> {
        Ok(BASE64.encode(serde_json::to_vec(&self.0)?))
    }
}

impl<T: serde::de::DeserializeOwned> Encoded<T> {
    /// The value a string from [`encode`](Self::encode) holds.
    pub fn decode(text: &str) -> serde_json::Result<Self> {
        let json = BASE64
            .decode(text)
            .map_err(<serde_json::Error as serde::de::Error>::custom)?;
        serde_json::from_slice(&json).map(Encoded)
    }
}

impl<T: serde::de::DeserializeOwned> FromStr for Encoded<T> {
    type Err = serde_json::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::decode(text)
    }
}

impl<T> From<T> for Encoded<T> {
    fn from(value: T) -> Self {
        Encoded(value)
    }
}

impl<T> Deref for Encoded<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> AsRef<T> for Encoded<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: serde::Serialize> serde::Serialize for Encoded<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let text = self.encode().map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&text)
    }
}

impl<'de, T: serde::de::DeserializeOwned> serde::Deserialize<'de> for Encoded<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = std::borrow::Cow::<'de, str>::deserialize(deserializer)?;
        Self::decode(&text).map_err(serde::de::Error::custom)
    }
}

impl<T> crate::Input for Encoded<T> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        String::reflectapi_input_type(schema)
    }
}

impl<T> crate::Output for Encoded<T> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        String::reflectapi_output_type(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Cursor {
        after: String,
        id: u64,
    }

    #[test]
    fn round_trips_as_a_url_safe_string() {
        let cursor = Encoded(Cursor {
            after: "ü?/+".into(),
            id: u64::MAX,
        });
        let json = serde_json::to_string(&cursor).unwrap();
        let text: String = serde_json::from_str(&json).unwrap();
        assert!(
            text.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "{text}"
        );
        let back: Encoded<Cursor> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cursor);
    }

    #[test]
    fn a_bad_string_is_an_error() {
        assert!(Encoded::<Cursor>::decode("not base64!").is_err());
        // Valid base64url of JSON that isn't a `Cursor`.
        let other = BASE64.encode(br#"{"x":1}"#);
        assert!(Encoded::<Cursor>::decode(&other).is_err());
    }

    #[test]
    fn reflects_as_a_string() {
        let mut typespace = crate::Typespace::default();
        let type_ref = <Encoded<Cursor> as crate::Output>::reflectapi_output_type(&mut typespace);
        assert_eq!(type_ref.name, "std::string::String");
    }

    #[cfg(feature = "msgpack")]
    #[test]
    fn is_a_string_in_msgpack_too() {
        let cursor = Encoded(Cursor {
            after: "a".into(),
            id: 1,
        });
        let bytes = rmp_serde::to_vec_named(&cursor).unwrap();
        let text: String = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(text, cursor.encode().unwrap());
        assert_eq!(
            rmp_serde::from_slice::<Encoded<Cursor>>(&bytes).unwrap(),
            cursor
        );
    }
}
