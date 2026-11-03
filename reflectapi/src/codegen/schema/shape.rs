//! The shape serde gives a struct's or a variant's fields.
//!
//! Serde's data model has four: a unit (no fields), a newtype (one unnamed
//! field, written as that field's value), a tuple (any other number of
//! unnamed fields, written as an array) and a struct (named fields, written
//! as an object). How a variant appears on the wire follows from its shape
//! and its enum's representation, so backends read the shape from here.

use reflectapi_schema::{Field, Fields};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Shape<'a> {
    /// `A`
    Unit,
    /// `A(T)`
    Newtype(&'a Field),
    /// `A(T, U)`, or `A()`, which is an empty array rather than a unit.
    Tuple(&'a [Field]),
    /// `A { x: T }`, or `A {}`, which is an empty object rather than a unit.
    Struct(&'a [Field]),
}

impl<'a> Shape<'a> {
    pub fn of(fields: &'a Fields) -> Self {
        match fields {
            Fields::None => Shape::Unit,
            Fields::Unnamed(fields) => match fields.as_slice() {
                [newtype] => Shape::Newtype(newtype),
                fields => Shape::Tuple(fields),
            },
            Fields::Named(fields) => Shape::Struct(fields),
        }
    }

    /// The fields in declaration order.
    pub fn fields(&self) -> &'a [Field] {
        match self {
            Shape::Unit => &[],
            Shape::Newtype(field) => std::slice::from_ref(*field),
            Shape::Tuple(fields) | Shape::Struct(fields) => fields,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str) -> Field {
        Field::new(name.into(), "u32".into())
    }

    #[test]
    fn one_unnamed_field_is_a_newtype_and_any_other_number_a_tuple() {
        let newtype = Fields::Unnamed(vec![field("0")]);
        assert!(matches!(Shape::of(&newtype), Shape::Newtype(f) if f.name == "0"));
        let pair = Fields::Unnamed(vec![field("0"), field("1")]);
        assert!(matches!(Shape::of(&pair), Shape::Tuple([_, _])));
        let empty = Fields::Unnamed(Vec::new());
        assert!(matches!(Shape::of(&empty), Shape::Tuple([])));
    }

    #[test]
    fn named_fields_are_a_struct_even_when_there_are_none() {
        let one = Fields::Named(vec![field("x")]);
        assert!(matches!(Shape::of(&one), Shape::Struct([f]) if f.name == "x"));
        let empty = Fields::Named(Vec::new());
        assert!(matches!(Shape::of(&empty), Shape::Struct([])));
        assert!(matches!(Shape::of(&Fields::None), Shape::Unit));
    }

    #[test]
    fn fields_lists_every_shape_in_declaration_order() {
        let newtype = Fields::Unnamed(vec![field("0")]);
        let names = |fields: &Fields| {
            Shape::of(fields)
                .fields()
                .iter()
                .map(|f| f.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&newtype), ["0"]);
        assert_eq!(
            names(&Fields::Named(vec![field("a"), field("b")])),
            ["a", "b"]
        );
        assert!(names(&Fields::None).is_empty());
    }
}
