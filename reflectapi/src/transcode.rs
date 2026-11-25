//! Moving a value from one serde format to another, without building it in
//! memory. [`Raw`](crate::Raw) uses it to convert between its JSON text and
//! MessagePack.

use std::cell::RefCell;

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{self, Serialize, SerializeMap, SerializeSeq, Serializer};

/// Writes the value `deserializer` reads into `serializer`. With `to_json`,
/// a value JSON can't represent is an error instead of being changed:
/// serde_json would write binary data as an array of numbers, and NaN or an
/// infinite number as `null`.
pub(crate) fn transcode<'de, D, S>(
    deserializer: D,
    serializer: S,
    to_json: bool,
) -> Result<S::Ok, S::Error>
where
    D: Deserializer<'de>,
    S: Serializer,
{
    Transcoder::new(deserializer, to_json).serialize(serializer)
}

fn s2d<E: ser::Error, F: de::Error>(error: E) -> F {
    F::custom(error)
}

fn d2s<E: de::Error, F: ser::Error>(error: E) -> F {
    F::custom(error)
}

/// A deserializer seen as a value to serialize, once.
struct Transcoder<D> {
    deserializer: RefCell<Option<D>>,
    to_json: bool,
}

impl<D> Transcoder<D> {
    fn new(deserializer: D, to_json: bool) -> Self {
        Transcoder {
            deserializer: RefCell::new(Some(deserializer)),
            to_json,
        }
    }
}

impl<'de, D: Deserializer<'de>> Serialize for Transcoder<D> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Some(deserializer) = self.deserializer.borrow_mut().take() else {
            return Err(ser::Error::custom("a value can only be transcoded once"));
        };
        deserializer
            .deserialize_any(ValueVisitor {
                serializer,
                to_json: self.to_json,
            })
            .map_err(d2s)
    }
}

struct ValueVisitor<S> {
    serializer: S,
    to_json: bool,
}

macro_rules! forward {
    ($($visit:ident($ty:ty) => $serialize:ident;)*) => {$(
        fn $visit<E: de::Error>(self, v: $ty) -> Result<Self::Value, E> {
            self.serializer.$serialize(v).map_err(s2d)
        }
    )*};
}

impl<'de, S: Serializer> Visitor<'de> for ValueVisitor<S> {
    type Value = S::Ok;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("any value")
    }

    forward! {
        visit_bool(bool) => serialize_bool;
        visit_i8(i8) => serialize_i8;
        visit_i16(i16) => serialize_i16;
        visit_i32(i32) => serialize_i32;
        visit_i64(i64) => serialize_i64;
        visit_i128(i128) => serialize_i128;
        visit_u8(u8) => serialize_u8;
        visit_u16(u16) => serialize_u16;
        visit_u32(u32) => serialize_u32;
        visit_u64(u64) => serialize_u64;
        visit_u128(u128) => serialize_u128;
        visit_char(char) => serialize_char;
        visit_str(&str) => serialize_str;
    }

    fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
        if self.to_json && !v.is_finite() {
            return Err(E::custom(format!("{v} has no JSON form")));
        }
        self.serializer.serialize_f32(v).map_err(s2d)
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        if self.to_json && !v.is_finite() {
            return Err(E::custom(format!("{v} has no JSON form")));
        }
        self.serializer.serialize_f64(v).map_err(s2d)
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        if self.to_json {
            return Err(E::custom("binary data has no JSON form"));
        }
        self.serializer.serialize_bytes(v).map_err(s2d)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.serializer.serialize_none().map_err(s2d)
    }

    fn visit_some<D2: Deserializer<'de>>(self, d: D2) -> Result<Self::Value, D2::Error> {
        self.serializer
            .serialize_some(&Transcoder::new(d, self.to_json))
            .map_err(s2d)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.serializer.serialize_unit().map_err(s2d)
    }

    fn visit_newtype_struct<D2: Deserializer<'de>>(self, d: D2) -> Result<Self::Value, D2::Error> {
        Transcoder::new(d, self.to_json)
            .serialize(self.serializer)
            .map_err(s2d)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut out = self
            .serializer
            .serialize_seq(seq.size_hint())
            .map_err(s2d)?;
        while seq
            .next_element_seed(Element {
                out: &mut out,
                to_json: self.to_json,
            })?
            .is_some()
        {}
        out.end().map_err(s2d)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut out = self
            .serializer
            .serialize_map(map.size_hint())
            .map_err(s2d)?;
        while map
            .next_key_seed(Entry {
                out: &mut out,
                to_json: self.to_json,
                key: true,
            })?
            .is_some()
        {
            map.next_value_seed(Entry {
                out: &mut out,
                to_json: self.to_json,
                key: false,
            })?;
        }
        out.end().map_err(s2d)
    }

    fn visit_enum<A: de::EnumAccess<'de>>(self, _: A) -> Result<Self::Value, A::Error> {
        // JSON and MessagePack never report an enum to `deserialize_any`.
        Err(de::Error::custom("an enum can't be transcoded"))
    }
}

struct Element<'a, S> {
    out: &'a mut S,
    to_json: bool,
}

impl<'de, S: SerializeSeq> DeserializeSeed<'de> for Element<'_, S> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        self.out
            .serialize_element(&Transcoder::new(d, self.to_json))
            .map_err(s2d)
    }
}

struct Entry<'a, S> {
    out: &'a mut S,
    to_json: bool,
    key: bool,
}

impl<'de, S: SerializeMap> DeserializeSeed<'de> for Entry<'_, S> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        let value = Transcoder::new(d, self.to_json);
        if self.key {
            self.out.serialize_key(&value).map_err(s2d)
        } else {
            self.out.serialize_value(&value).map_err(s2d)
        }
    }
}
