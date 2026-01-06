use std::fmt;

#[derive(Debug)]
pub struct ValidationError {
    pub pointer: ValidationPointer,
    pub message: String,
}

impl ValidationError {
    pub fn new(pointer: ValidationPointer, message: String) -> ValidationError {
        ValidationError { pointer, message }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.pointer, self.message)
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug)]
pub enum ValidationPointer {
    Field {
        type_name: String,
        name: String,
    },
    Variant {
        type_name: String,
        name: String,
    },
    VariantField {
        type_name: String,
        variant_name: String,
        name: String,
    },
    Type(String),
    Function(String),
}

impl fmt::Display for ValidationPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationPointer::Field { type_name, name } => write!(f, "{type_name}.{name}"),
            ValidationPointer::Variant { type_name, name } => write!(f, "{type_name}::{name}"),
            ValidationPointer::VariantField {
                type_name,
                variant_name,
                name,
            } => write!(f, "{type_name}.{variant_name}.{name}"),
            ValidationPointer::Type(name) => write!(f, "{name}"),
            ValidationPointer::Function(name) => write!(f, "{name}"),
        }
    }
}
