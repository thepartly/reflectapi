use std::ops::Deref;

/// A three-state enum that represents a value that can be present (`Some`),
/// explicitly absent (`None`), a.k.a null, or not provided at all (`Undefined`).
///
/// This is particularly useful for distinguishing between a field that was intentionally
/// set to `null` versus a field that was omitted from a request or data structure,
/// a common pattern in APIs (e.g., JSON `null` vs. a missing key).
///
/// It is often used with `serde`'s `#[serde(default)]` and `#[serde(skip_serializing_if = "...")]`
/// attributes to handle optional fields in a more precise way than `Option<T>`.
///
/// # Example
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use reflectapi::Option;
///
/// #[derive(Serialize, Deserialize, Default)]
/// struct UserPatch {
///     #[serde(default, skip_serializing_if = "Option::is_undefined")]
///     name: Option<String>,
///     #[serde(default, skip_serializing_if = "Option::is_undefined")]
///     email: Option<Option<String>>,
/// }
///
/// // User wants to update their name, but not their email.
/// // The `email` field is omitted from the JSON.
/// let patch_json = r#"{"name": "Jane Doe"}"#;
/// let patch: UserPatch = serde_json::from_str(patch_json).unwrap();
/// assert_eq!(patch.name, Option::Some("Jane Doe".to_string()));
/// assert!(patch.email.is_undefined());
///
/// // User wants to clear their email, setting it to null.
/// let patch_json_null = r#"{"email": null}"#;
/// let patch_null: UserPatch = serde_json::from_str(patch_json_null).unwrap();
/// assert!(patch_null.name.is_undefined());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Copy)]
pub enum Option<T> {
    /// The value was not provided. This is the default state.
    /// When used with `#[serde(default)]`, this variant will be used for missing fields.
    #[default]
    Undefined,
    /// The value was provided and is explicitly `None` (e.g., `null` in JSON).
    None,
    /// The value was provided and is present.
    Some(T),
}

impl<T> Option<T> {
    /// Returns `true` if the `Option` is `Undefined`.
    pub fn is_undefined(&self) -> bool {
        matches!(self, Option::Undefined)
    }

    /// Returns `true` if the `Option` is `None`.
    pub fn is_none(&self) -> bool {
        matches!(self, Option::None)
    }

    /// Returns `true` if the `Option` is `None` or `Undefined`.
    pub fn is_none_or_undefined(&self) -> bool {
        matches!(self, Option::None | Option::Undefined)
    }

    /// Returns `true` if the `Option` is `Some`.
    pub fn is_some(&self) -> bool {
        matches!(self, Option::Some(_))
    }

    /// Converts from `Option<T>` to `Option<&T>`.
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Option::Undefined => Option::Undefined,
            Option::None => Option::None,
            Option::Some(value) => Option::Some(value),
        }
    }

    /// Converts the `Option<T>` into a standard `Option<T>`.
    ///
    /// Note: This is a lossy conversion, as both `Undefined` and `None`
    /// are mapped to `Option::None`.
    pub fn into_option(self) -> std::option::Option<T> {
        match self {
            Option::Undefined => None,
            Option::None => None,
            Option::Some(value) => Some(value),
        }
    }

    /// Converts the `Option<T>` into a standard `Option<T>`.
    /// Returning `default` if the value is `Undefined`.
    pub fn into_option_or(self, default: T) -> std::option::Option<T> {
        match self {
            Option::Undefined => Some(default),
            Option::None => None,
            Option::Some(value) => Some(value),
        }
    }

    /// Converts a reference to a `Option<T>` into an `Option<&T>`.
    ///
    /// Note: This is a lossy conversion, as both `Undefined` and `None`
    /// are mapped to `Option::None`.
    pub fn as_option(&self) -> std::option::Option<&T> {
        match self {
            Option::Undefined => None,
            Option::None => None,
            Option::Some(value) => Some(value),
        }
    }

    /// "Unfolds" the `Option<&T>` into a nested `Option<Option<&T>>`.
    ///
    /// This is a lossless conversion that preserves all three states:
    /// - `Option::Undefined` -> `None`
    /// - `Option::None`      -> `Some(None)`
    /// - `Option::Some(v)`   -> `Some(Some(v))`
    pub fn unfold(&self) -> std::option::Option<std::option::Option<&T>> {
        match self {
            Option::Undefined => None,
            Option::None => Some(std::option::Option::None),
            Option::Some(value) => Some(std::option::Option::Some(value)),
        }
    }

    /// "Folds" a nested `Option<Option<T>>` into a `Option<T>`.
    ///
    /// This is the inverse of `unfold` and is also lossless:
    /// - `None`           -> `Option::Undefined`
    /// - `Some(None)`     -> `Option::None`
    /// - `Some(Some(v))`  -> `Option::Some(v)`
    pub fn fold(source: std::option::Option<std::option::Option<T>>) -> Self {
        match source {
            None => Option::Undefined,
            Some(None) => Option::None,
            Some(Some(value)) => Option::Some(value),
        }
    }

    /// Maps a `Option<T>` to `Option<U>` by applying a function to a
    /// contained `Some` value, leaving `Undefined` and `None` values untouched.
    pub fn map<U, F>(self, f: F) -> Option<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Option::Undefined => Option::Undefined,
            Option::None => Option::None,
            Option::Some(value) => Option::Some(f(value)),
        }
    }

    /// Converts a `Option<T>` to a `Option<&T::Target>` where `T` implements `Deref`.
    pub fn as_deref(&self) -> Option<&T::Target>
    where
        T: Deref,
    {
        match self.as_ref() {
            Option::Undefined => Option::Undefined,
            Option::None => Option::None,
            Option::Some(t) => Option::Some(t.deref()),
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

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

fn reflectapi_type_option(schema: &mut crate::Typespace) -> String {
    let type_name = "reflectapi::Option";
    if schema.reserve_type(type_name) {
        let mut type_def = crate::Enum::new(type_name.into());
        type_def.parameters.push("T".into());
        type_def.description = "Undefinable Option type".into();
        type_def.representation = crate::Representation::None;

        let mut variant = crate::Variant::new("Undefined".into());
        variant.description = "The value is missing, i.e. undefined in JavaScript".into();
        type_def.variants.push(variant);

        let mut variant = crate::Variant::new("None".into());
        variant.description =
            "The value is provided but set to none, i.e. null in JavaScript".into();
        type_def.variants.push(variant);

        let mut variant = crate::Variant::new("Some".into());
        variant.description = "The value is provided and set to some value".into();
        variant.fields = crate::Fields::Unnamed(vec![crate::Field::new("0".into(), "T".into())]);

        type_def.variants.push(variant);

        schema.insert_type(type_def.into());
    }
    type_name.into()
}

impl<T: crate::Input> crate::Input for crate::Option<T> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_option(schema),
            vec![T::reflectapi_input_type(schema)],
        )
    }
}

impl<T: crate::Output> crate::Output for crate::Option<T> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_option(schema),
            vec![T::reflectapi_output_type(schema)],
        )
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
