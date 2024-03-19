#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Option<T> {
    #[default]
    Undefined,
    None,
    Some(T),
}

impl<T> Option<T> {
    pub fn is_undefined(&self) -> bool {
        matches!(self, Option::Undefined)
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Option::None)
    }

    pub fn is_none_or_undefined(&self) -> bool {
        matches!(self, Option::None | Option::Undefined)
    }

    pub fn is_some(&self) -> bool {
        matches!(self, Option::Some(_))
    }

    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Option::Undefined => Option::Undefined,
            Option::None => Option::None,
            Option::Some(value) => Option::Some(value),
        }
    }

    pub fn as_option(&self) -> std::option::Option<&T> {
        match self {
            Option::Undefined => None,
            Option::None => None,
            Option::Some(value) => Some(value),
        }
    }
}

impl<T> From<std::option::Option<T>> for Option<T> {
    fn from(option: std::option::Option<T>) -> Self {
        match option {
            Some(value) => Option::Some(value),
            None => Option::None,
        }
    }
}

impl<T> From<Option<T>> for std::option::Option<T> {
    fn from(val: Option<T>) -> Self {
        match val {
            Option::Undefined => None,
            Option::None => None,
            Option::Some(value) => Some(value),
        }
    }
}

impl<T: serde::Serialize> serde::Serialize for Option<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Option::Undefined => serializer.serialize_none(),
            Option::None => serializer.serialize_none(),
            Option::Some(v) => serializer.serialize_some(v),
        }
    }
}

impl<'de, T: serde::de::Deserialize<'de>> serde::Deserialize<'de> for Option<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UndefinableOptionVisitor<V> {
            marker: std::marker::PhantomData<V>,
        }

        impl<'de, V> serde::de::Visitor<'de> for UndefinableOptionVisitor<V>
        where
            V: serde::Deserialize<'de>,
        {
            type Value = Option<V>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an option or undefined")
            }

            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(Option::None)
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(Option::None)
            }

            fn visit_some<D: serde::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                serde::Deserialize::deserialize(deserializer).map(Option::Some)
            }
        }

        deserializer.deserialize_option(UndefinableOptionVisitor {
            marker: std::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {

    #[derive(serde::Serialize, serde::Deserialize, Default)]
    struct Test {
        #[serde(default, skip_serializing_if = "crate::Option::is_undefined")]
        value: crate::Option<String>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(untagged)]
    enum TestUntagged {
        Variant1(Test),
    }

    #[test]
    fn test_undefined() {
        let data = Test::default();
        assert!(data.value.is_undefined());
        assert!(!data.value.is_none());
        assert!(!data.value.is_some());
        assert!(data.value.is_none_or_undefined());

        let serialized = serde_json::to_string(&data).unwrap();
        assert_eq!(serialized, "{}");

        let deserialized: Test = serde_json::from_str(&serialized).unwrap();
        assert!(deserialized.value.is_undefined());
    }

    #[test]
    fn test_undefined_untagged() {
        let data = Test::default();
        assert!(data.value.is_undefined());
        assert!(!data.value.is_none());
        assert!(!data.value.is_some());
        assert!(data.value.is_none_or_undefined());

        let serialized = serde_json::to_string(&data).unwrap();
        assert_eq!(serialized, "{}");

        let deserialized: TestUntagged = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            TestUntagged::Variant1(deserialized) => {
                assert!(deserialized.value.is_undefined());
                assert!(!deserialized.value.is_none());
            }
        }
    }

    #[test]
    fn test_none() {
        let data = Test {
            value: crate::Option::None,
        };
        assert!(!data.value.is_undefined());
        assert!(data.value.is_none());
        assert!(!data.value.is_some());
        assert!(data.value.is_none_or_undefined());

        let serialized = serde_json::to_string(&data).unwrap();
        assert_eq!(serialized, "{\"value\":null}");

        let deserialized: Test = serde_json::from_str(&serialized).unwrap();
        assert!(!deserialized.value.is_undefined());
        assert!(deserialized.value.is_none());
    }

    #[test]
    fn test_none_untagged() {
        let data = Test {
            value: crate::Option::None,
        };
        assert!(!data.value.is_undefined());
        assert!(data.value.is_none());
        assert!(!data.value.is_some());
        assert!(data.value.is_none_or_undefined());

        let serialized = serde_json::to_string(&data).unwrap();
        assert_eq!(serialized, "{\"value\":null}");

        let deserialized: TestUntagged = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            TestUntagged::Variant1(deserialized) => {
                assert!(!deserialized.value.is_undefined());
                assert!(deserialized.value.is_none());
            }
        }
    }

    #[test]
    fn test_some() {
        let data = Test {
            value: crate::Option::Some(String::from("test")),
        };
        assert!(!data.value.is_undefined());
        assert!(!data.value.is_none());
        assert!(data.value.is_some());
        assert!(!data.value.is_none_or_undefined());

        let serialized = serde_json::to_string(&data).unwrap();
        assert_eq!(serialized, "{\"value\":\"test\"}");

        let deserialized: Test = serde_json::from_str(&serialized).unwrap();
        assert!(!deserialized.value.is_undefined());
        assert!(!deserialized.value.is_none());
        assert!(deserialized.value.is_some());
        assert_eq!(
            deserialized.value,
            crate::Option::from(Some(String::from("test")))
        );
    }

    #[test]
    fn test_some_invalid() {
        let serialized = String::from("{\"value\":1}");

        if let Err(e) = serde_json::from_str::<Test>(&serialized) {
            assert_eq!(
                e.to_string(),
                "invalid type: integer `1`, expected a string at line 1 column 10"
            );
        } else {
            panic!("expected error");
        }
    }
}
