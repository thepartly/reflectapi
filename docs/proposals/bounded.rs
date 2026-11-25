use core::fmt;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::ops::Deref;

/// A value with a length, for [`Bounded`].
pub trait Length {
    /// The length [`Bounded`] checks: characters for strings, entries for
    /// collections.
    fn length(&self) -> usize;
}

impl Length for String {
    // Characters, not bytes: limits shown to people count characters.
    fn length(&self) -> usize {
        self.chars().count()
    }
}

impl<T> Length for Vec<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> Length for VecDeque<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T, S> Length for HashSet<T, S> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<T> Length for BTreeSet<T> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<K, V, S> Length for HashMap<K, V, S> {
    fn length(&self) -> usize {
        self.len()
    }
}

impl<K, V> Length for BTreeMap<K, V> {
    fn length(&self) -> usize {
        self.len()
    }
}

/// A string or collection whose length is within `MIN..=MAX`, inclusive.
///
/// On the wire it is the `T` it wraps. Deserializing a value of another
/// length fails, so a request carrying one is rejected with HTTP 400 before
/// the handler runs. Strings count characters, collections count entries.
///
/// The schema keeps the bounds, and every generated client checks them before
/// sending: TypeScript through `bounded(value, MIN, MAX)`, Python through
/// Pydantic's `min_length`/`max_length`, and a Rust client by using this type.
/// OpenAPI gets the matching length keywords.
///
/// ```rust
/// use reflectapi::{Bounded, BoundedMax};
///
/// #[derive(serde::Deserialize, reflectapi::Input)]
/// struct CreateTags {
///     /// Up to 50 tags, each 1 to 32 characters long.
///     tags: BoundedMax<Vec<Bounded<String, 1, 32>>, 50>,
/// }
///
/// assert!(serde_json::from_str::<CreateTags>(r#"{"tags":["red"]}"#).is_ok());
/// assert!(serde_json::from_str::<CreateTags>(r#"{"tags":[""]}"#).is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Bounded<T, const MIN: usize, const MAX: usize>(T);

/// [`Bounded`] with only an upper bound.
pub type BoundedMax<T, const MAX: usize> = Bounded<T, 0, MAX>;

/// A length outside a [`Bounded`]'s bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundsError {
    pub length: usize,
    pub min: usize,
    pub max: usize,
}

impl fmt::Display for BoundsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "length {} out of bounds (min {}, max {})",
            self.length, self.min, self.max
        )
    }
}

impl std::error::Error for BoundsError {}

impl<T: Length, const MIN: usize, const MAX: usize> Bounded<T, MIN, MAX> {
    /// The value, if its length is within the bounds.
    pub fn new(value: T) -> Result<Self, BoundsError> {
        const {
            assert!(MIN <= MAX, "Bounded declared with MIN > MAX");
        }
        let length = value.length();
        if length < MIN || length > MAX {
            return Err(BoundsError {
                length,
                min: MIN,
                max: MAX,
            });
        }
        Ok(Self(value))
    }
}

impl<T, const MIN: usize, const MAX: usize> Bounded<T, MIN, MAX> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, const MIN: usize, const MAX: usize> Bounded<Vec<T>, MIN, MAX> {
    /// From an array whose length is checked at compile time.
    pub fn from_array<const N: usize>(items: [T; N]) -> Self {
        const {
            assert!(MIN <= N && N <= MAX, "array length out of Bounded bounds");
        }
        Self(items.into())
    }

    /// `items` split into consecutive chunks of at most `MAX` items, for
    /// sending a long list in several requests.
    pub fn chunks(items: &[T]) -> Vec<Self>
    where
        T: Clone,
    {
        const {
            assert!(MAX > 0, "cannot chunk into Bounded with MAX == 0");
            assert!(MIN <= 1, "a short final chunk cannot satisfy MIN > 1");
        }
        items
            .chunks(MAX)
            .map(|chunk| Self(chunk.to_vec()))
            .collect()
    }
}

impl<T, const MIN: usize, const MAX: usize> Deref for Bounded<T, MIN, MAX> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const MIN: usize, const MAX: usize> AsRef<T> for Bounded<T, MIN, MAX> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T, const MIN: usize, const MAX: usize> TryFrom<Vec<T>> for Bounded<Vec<T>, MIN, MAX> {
    type Error = BoundsError;

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MIN: usize, const MAX: usize> TryFrom<String> for Bounded<String, MIN, MAX> {
    type Error = BoundsError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MIN: usize, const MAX: usize> TryFrom<&str> for Bounded<String, MIN, MAX> {
    type Error = BoundsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.to_owned())
    }
}

fn reflectapi_type_bounded<const MIN: usize, const MAX: usize>(
    schema: &mut crate::Typespace,
    inner: crate::TypeReference,
) -> crate::TypeReference {
    let type_name = "reflectapi::Bounded";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "A string or collection whose length is within MIN..=MAX, inclusive: characters for strings, entries for collections. A request with another length is rejected with HTTP 400.".into(),
            vec!["T".into(), "MIN".into(), "MAX".into()],
            Some(crate::TypeReference::new("T", Vec::new())),
        );
        schema.insert_type(type_def.into());
    }
    crate::TypeReference::new(
        type_name,
        vec![inner, MIN.to_string().into(), MAX.to_string().into()],
    )
}

impl<T: crate::Input, const MIN: usize, const MAX: usize> crate::Input for Bounded<T, MIN, MAX> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        let inner = T::reflectapi_input_type(schema);
        reflectapi_type_bounded::<MIN, MAX>(schema, inner)
    }
}

impl<T: crate::Output, const MIN: usize, const MAX: usize> crate::Output for Bounded<T, MIN, MAX> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        let inner = T::reflectapi_output_type(schema);
        reflectapi_type_bounded::<MIN, MAX>(schema, inner)
    }
}

impl<T: serde::Serialize, const MIN: usize, const MAX: usize> serde::Serialize
    for Bounded<T, MIN, MAX>
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T, const MIN: usize, const MAX: usize> serde::Deserialize<'de> for Bounded<T, MIN, MAX>
where
    T: serde::Deserialize<'de> + Length,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(T::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_in_range() {
        let bounded: Bounded<Vec<u32>, 1, 3> = serde_json::from_str("[1, 2]").unwrap();
        assert_eq!(*bounded, vec![1, 2]);
        assert_eq!(bounded.into_inner(), vec![1, 2]);
    }

    #[test]
    fn rejects_over_max() {
        let err = serde_json::from_str::<Bounded<Vec<u32>, 0, 2>>("[1, 2, 3]").unwrap_err();
        assert!(
            err.to_string()
                .contains("length 3 out of bounds (min 0, max 2)"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn rejects_under_min() {
        let err = serde_json::from_str::<Bounded<String, 1, 10>>("\"\"").unwrap_err();
        assert!(
            err.to_string()
                .contains("length 0 out of bounds (min 1, max 10)"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn strings_count_characters_not_bytes() {
        // One character, four bytes.
        let bounded: Bounded<String, 1, 1> = serde_json::from_str("\"🦀\"").unwrap();
        assert_eq!(*bounded, "🦀");
    }

    #[test]
    fn maps_count_entries() {
        let err = serde_json::from_str::<BoundedMax<BTreeMap<String, u8>, 1>>(r#"{"a":1,"b":2}"#)
            .unwrap_err();
        assert!(err.to_string().contains("length 2"), "{err}");
    }

    #[test]
    fn reflects_as_a_primitive_with_the_bounds_as_arguments() {
        let mut typespace = crate::Typespace::default();
        let type_ref =
            <BoundedMax<Vec<u32>, 500> as crate::Input>::reflectapi_input_type(&mut typespace);
        assert_eq!(type_ref.name, "reflectapi::Bounded");
        let args: Vec<&str> = type_ref.arguments.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(args, ["std::vec::Vec", "0", "500"]);

        // Codegens that don't know it see the wrapped type.
        let fallback = type_ref.fallback_once(&typespace).unwrap();
        assert_eq!(fallback.name, "std::vec::Vec");

        // A second instantiation reuses the definition.
        let type_ref =
            <Bounded<String, 1, 10> as crate::Input>::reflectapi_input_type(&mut typespace);
        let args: Vec<&str> = type_ref.arguments.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(args, ["std::string::String", "1", "10"]);
    }

    #[test]
    fn from_array_checks_the_length_at_compile_time() {
        let bounded: BoundedMax<Vec<u32>, 3> = Bounded::from_array([1, 2]);
        assert_eq!(*bounded, vec![1, 2]);
        // `BoundedMax::<Vec<u32>, 3>::from_array([1, 2, 3, 4])` doesn't compile.
    }

    #[test]
    fn chunks_fit_the_bound() {
        let chunks = BoundedMax::<Vec<u32>, 2>::chunks(&[1, 2, 3, 4, 5]);
        let lengths: Vec<usize> = chunks.iter().map(|c| c.len()).collect();
        assert_eq!(lengths, [2, 2, 1]);
    }

    #[test]
    fn serializes_as_the_wrapped_value() {
        let bounded: BoundedMax<Vec<u32>, 5> = Bounded::new(vec![1, 2]).unwrap();
        assert_eq!(serde_json::to_string(&bounded).unwrap(), "[1,2]");
    }
}
