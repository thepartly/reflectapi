use std::fmt;

#[derive(Debug)]
pub struct ValidationErrors(pub Vec<ValidationError>);

impl IntoIterator for ValidationErrors {
    type Item = ValidationError;
    type IntoIter = std::vec::IntoIter<ValidationError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for err in &self.0 {
            writeln!(f, "{err}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

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
            ValidationPointer::Field { type_name, name } => write!(f, "{}.{}", type_name, name),
            ValidationPointer::Variant { type_name, name } => write!(f, "{}.{}", type_name, name),
            ValidationPointer::VariantField {
                type_name,
                variant_name,
                name,
            } => write!(f, "{}.{}.{}", type_name, variant_name, name),
            ValidationPointer::Type(name) => write!(f, "{}", name),
            ValidationPointer::Function(name) => write!(f, "{}", name),
        }
    }
}
