//! Types whose schema reference carries a length range: `reflectapi::Bounded`
//! and fixed-size arrays. One place decides what counts as one and what its
//! bounds are; each generator only decides how to write them.

use crate::TypeReference;

/// What a length counts, which decides the keywords and checks a generator
/// writes for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LengthKind {
    /// Characters of a string.
    Characters,
    /// Items of an array or set.
    Items,
    /// Entries of a map.
    Entries,
}

/// A reference with a length range, unpacked.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LengthBounds<'a> {
    /// The collection the bounds apply to: `T` of `Bounded<T, MIN, MAX>`. `None`
    /// for a fixed-size array, which is the collection itself: render the
    /// reference as without bounds, through its fallback.
    pub inner: Option<&'a TypeReference>,
    pub min: u64,
    pub max: u64,
    /// `None` when `T` isn't a type whose length a client can check, such as
    /// a user type implementing `reflectapi::Length`: write no checks.
    pub kind: Option<LengthKind>,
}

/// The length range of `type_ref`, if it has one.
pub(crate) fn length_bounds(type_ref: &TypeReference) -> Option<LengthBounds<'_>> {
    match type_ref.name.as_str() {
        "reflectapi::Bounded" => {
            let [inner, min, max] = type_ref.arguments.as_slice() else {
                return None;
            };
            Some(LengthBounds {
                inner: Some(inner),
                min: min.name.parse().ok()?,
                max: max.name.parse().ok()?,
                kind: length_kind(&inner.name),
            })
        }
        "std::array::Array" => {
            let [_, n] = type_ref.arguments.as_slice() else {
                return None;
            };
            let n = n.name.parse().ok()?;
            Some(LengthBounds {
                inner: None,
                min: n,
                max: n,
                kind: Some(LengthKind::Items),
            })
        }
        _ => None,
    }
}

fn length_kind(name: &str) -> Option<LengthKind> {
    match name {
        "std::string::String" => Some(LengthKind::Characters),
        "std::vec::Vec"
        | "std::array::Array"
        | "std::collections::HashSet"
        | "std::collections::BTreeSet"
        | "indexmap::IndexSet" => Some(LengthKind::Items),
        "std::collections::HashMap" | "std::collections::BTreeMap" | "indexmap::IndexMap" => {
            Some(LengthKind::Entries)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(name: &str, arguments: Vec<TypeReference>) -> TypeReference {
        TypeReference::new(name, arguments)
    }

    #[test]
    fn unpacks_bounded() {
        let string = r("std::string::String", vec![]);
        let type_ref = r(
            "reflectapi::Bounded",
            vec![string.clone(), r("1", vec![]), r("32", vec![])],
        );
        assert_eq!(
            length_bounds(&type_ref),
            Some(LengthBounds {
                inner: Some(&string),
                min: 1,
                max: 32,
                kind: Some(LengthKind::Characters),
            })
        );
    }

    #[test]
    fn unpacks_fixed_size_arrays_as_exact_lengths() {
        let type_ref = r("std::array::Array", vec![r("u8", vec![]), r("3", vec![])]);
        let bounds = length_bounds(&type_ref).unwrap();
        assert_eq!((bounds.inner, bounds.min, bounds.max), (None, 3, 3));
        assert_eq!(bounds.kind, Some(LengthKind::Items));
    }

    #[test]
    fn a_user_length_type_has_bounds_but_no_kind() {
        let type_ref = r(
            "reflectapi::Bounded",
            vec![r("my::Tags", vec![]), r("0", vec![]), r("5", vec![])],
        );
        assert_eq!(length_bounds(&type_ref).unwrap().kind, None);
    }

    #[test]
    fn malformed_references_have_no_bounds() {
        let string = r("std::string::String", vec![]);
        // Too few arguments, and a bound that isn't a number.
        assert_eq!(
            length_bounds(&r("reflectapi::Bounded", vec![string.clone()])),
            None
        );
        let not_a_number = r(
            "reflectapi::Bounded",
            vec![string, r("MIN", vec![]), r("5", vec![])],
        );
        assert_eq!(length_bounds(&not_a_number), None);
        assert_eq!(length_bounds(&r("std::vec::Vec", vec![])), None);
    }
}
