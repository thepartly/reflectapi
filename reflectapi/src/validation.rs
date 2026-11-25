use std::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub struct ValidationError {
    pub pointer: ValidationPointer,
    pub message: String,
    /// The id of the rule the error enforces, such as `api-call-name-form`
    /// for a rule of the API design standard. A validator can then be
    /// relaxed for one rule by filtering on it.
    pub rule: Option<String>,
}

impl ValidationError {
    pub fn new(pointer: ValidationPointer, message: String) -> ValidationError {
        ValidationError {
            pointer,
            message,
            rule: None,
        }
    }

    /// Names the rule the error enforces.
    pub fn with_rule(mut self, rule: impl Into<String>) -> ValidationError {
        self.rule = Some(rule.into());
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.pointer, self.message)
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug)]
#[non_exhaustive]
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
