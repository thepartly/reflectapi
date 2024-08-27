mod format;
pub mod openapi;
pub mod rust;
pub mod typescript;

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hasher},
    path::PathBuf,
};

use reflectapi_schema::{Enum, Field, Struct, TypeReference, Variant};

use self::format::format_with;

#[derive(Debug, Default)]
pub struct Config {
    /// Attempt to format the generated code. Will give up if no formatter is found.
    pub format: bool,
    /// Typecheck the generated code. Will ignore if the typechecker is not available.
    pub typecheck: bool,
    pub shared_modules: Vec<String>,
}

fn tmp_path(src: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    hasher.write(src.as_bytes());
    let hash = hasher.finish();

    std::env::temp_dir().join(format!("reflectapi-{hash}"))
}

trait Instantiate {
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

trait Substitute {
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

impl Substitute for Field {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self {
        Field {
            type_ref: self.type_ref.subst(subst),
            ..self
        }
    }
}

impl Substitute for Variant {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Variant {
        Self {
            fields: self.fields.into_iter().map(|f| f.subst(subst)).collect(),
            ..self
        }
    }
}

impl Instantiate for Struct {
    /// Return a new non-generic `Struct` with each type parameter substituted with a type
    fn instantiate(self, type_args: &[TypeReference]) -> Self {
        assert_eq!(
            self.parameters.len(),
            type_args.len(),
            "expected {} type arguments, got {}",
            self.parameters.len(),
            type_args.len()
        );

        let subst = self
            .parameters
            .iter()
            .map(|p| p.name.to_owned())
            .zip(type_args.iter().cloned())
            .collect::<HashMap<_, _>>();

        Self {
            parameters: vec![],
            fields: self.fields.into_iter().map(|f| f.subst(&subst)).collect(),
            ..self
        }
    }
}

impl Instantiate for Enum {
    /// Return a new non-generic `Enum` with each type parameter substituted with a type
    fn instantiate(self, type_args: &[TypeReference]) -> Self {
        assert_eq!(
            self.parameters.len(),
            type_args.len(),
            "expected {} type arguments, got {}",
            self.parameters.len(),
            type_args.len()
        );

        let subst = self
            .parameters
            .iter()
            .map(|p| p.name.to_owned())
            .zip(type_args.iter().cloned())
            .collect::<HashMap<_, _>>();

        Self {
            parameters: vec![],
            variants: self.variants.into_iter().map(|v| v.subst(&subst)).collect(),
            ..self
        }
    }
}
