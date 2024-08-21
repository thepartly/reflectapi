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

trait Subst {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self;
}

trait Instantiate {
    fn instantiate(self, args: &[TypeReference]) -> Self;
}

impl Subst for TypeReference {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self {
        match subst.get(&self.name) {
            Some(ty) => {
                assert!(
                    self.arguments.is_empty(),
                    "type parameter cannot have type arguments (no HKTs)"
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

impl Subst for Field {
    fn subst(self, subst: &HashMap<String, TypeReference>) -> Self {
        Field {
            type_ref: self.type_ref.subst(subst),
            ..self
        }
    }
}

impl Subst for Variant {
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
