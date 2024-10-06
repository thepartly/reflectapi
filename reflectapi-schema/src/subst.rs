use std::collections::HashMap;

use crate::{Enum, Field, Fields, Struct, TypeParameter, TypeReference, Variant};

#[doc(hidden)]
pub fn mk_subst(
    parameters: &[TypeParameter],
    args: &[TypeReference],
) -> HashMap<String, TypeReference> {
    assert_eq!(
        parameters.len(),
        args.len(),
        "expected {} type arguments, got {}",
        parameters.len(),
        args.len()
    );

    parameters
        .iter()
        .map(|p| p.name.to_owned())
        .zip(args.iter().cloned())
        .collect()
}

pub trait Instantiate {
    /// Apply type arguments to a generic struct or enum. This should return a non-generic
    /// instance with all type parameters substituted with the matching type arguments.
    ///
    /// Example:
    /// Given a generic struct `struct Foo<T, U> { a: T, b: U }` and type arguments `[i32,
    /// String]`, the `instantiate(struct Foo<T, U>, [i32, String])` should be a non-generic struct `struct Foo { a: i32, b: String }`.
    /// This is implemented by generating a substitution map from the type parameters to the type
    /// argument `T -> i32, U -> String` and then substituting each occurence of the type parameter with the type argument.
    fn instantiate(self, args: &[TypeReference]) -> Self;
}

#[doc(hidden)]
pub trait Substitute {
    /// The important implementation of this is `impl Substitute for TypeReference`.
    /// All other implementations just recursively call `subst` on their relevant fields which
    /// contain `TypeReference`s.
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self;
}

impl Substitute for TypeReference {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self {
        match subst.get(&self.name) {
            Some(ty) => {
                assert!(
                    self.arguments.is_empty(),
                    "type parameter cannot have type arguments"
                );
                ty.clone()
            }
            None => TypeReference {
                name: self.name,
                arguments: self.arguments.into_iter().map(|a| a.subst(subst)).collect(),
            },
        }
    }
}

impl Substitute for Fields {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self {
        match self {
            Fields::Named(fields) => {
                Fields::Named(fields.into_iter().map(|f| f.subst(subst)).collect())
            }
            Fields::Unnamed(fields) => {
                Fields::Unnamed(fields.into_iter().map(|f| f.subst(subst)).collect())
            }
            Fields::None => Fields::None,
        }
    }
}

impl Substitute for Field {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self {
        Field {
            type_ref: self.type_ref.subst(subst),
            ..self
        }
    }
}

impl Substitute for Variant {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self {
        Self {
            fields: self.fields.subst(subst),
            ..self
        }
    }
}

impl Instantiate for Struct {
    /// Return a new non-generic `Struct` with each type parameter substituted with a type
    fn instantiate(self, type_args: &[TypeReference]) -> Self {
        let subst = mk_subst(&self.parameters, type_args);

        Self {
            parameters: vec![],
            fields: self.fields.subst(&subst),
            ..self
        }
    }
}

impl Instantiate for Enum {
    /// Return a new non-generic `Enum` with each type parameter substituted with a type
    fn instantiate(self, type_args: &[TypeReference]) -> Self {
        let subst = mk_subst(&self.parameters, type_args);

        Self {
            parameters: vec![],
            variants: self.variants.into_iter().map(|v| v.subst(&subst)).collect(),
            ..self
        }
    }
}
