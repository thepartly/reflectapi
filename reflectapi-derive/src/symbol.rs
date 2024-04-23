use std::fmt::{self, Display};
use syn::{Ident, Path};

#[derive(Copy, Clone)]
pub struct Symbol(&'static str);

pub const REFLECT: Symbol = Symbol("reflect");
pub const DOC: Symbol = Symbol("doc");

pub const TYPE: Symbol = Symbol("type");
pub const INPUT_TYPE: Symbol = Symbol("input_type");
pub const OUTPUT_TYPE: Symbol = Symbol("output_type");

pub const TRANSFORM: Symbol = Symbol("transform");
pub const INPUT_TRANSFORM: Symbol = Symbol("input_transform");
pub const OUTPUT_TRANSFORM: Symbol = Symbol("output_transform");

pub const SKIP: Symbol = Symbol("skip");
pub const INPUT_SKIP: Symbol = Symbol("input_skip");
pub const OUTPUT_SKIP: Symbol = Symbol("output_skip");

pub const DISCRIMINANT: Symbol = Symbol("discriminant");

impl PartialEq<Symbol> for Ident {
    fn eq(&self, word: &Symbol) -> bool {
        self == word.0
    }
}

impl<'a> PartialEq<Symbol> for &'a Ident {
    fn eq(&self, word: &Symbol) -> bool {
        *self == word.0
    }
}

impl PartialEq<Symbol> for Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl<'a> PartialEq<Symbol> for &'a Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl Display for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}
