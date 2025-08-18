/// Centralized Python naming conventions
/// 
/// This module is the single source of truth for all Python identifier generation.
/// All naming decisions are made in the semantic layer and should use these functions.
/// The transform and render stages should receive already-correct names and do zero further transformation.

use std::collections::HashSet;

/// Python reserved keywords that require escaping
const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", 
    "class", "continue", "def", "del", "elif", "else", "except", "finally", 
    "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
    "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

/// Python built-in functions that we should avoid shadowing
const PYTHON_BUILTINS: &[&str] = &[
    "abs", "all", "any", "ascii", "bin", "bool", "bytes", "callable", "chr",
    "classmethod", "compile", "complex", "delattr", "dict", "dir", "divmod",
    "enumerate", "eval", "exec", "filter", "float", "format", "frozenset",
    "getattr", "globals", "hasattr", "hash", "help", "hex", "id", "input",
    "int", "isinstance", "issubclass", "iter", "len", "list", "locals", "map",
    "max", "memoryview", "min", "next", "object", "oct", "open", "ord", "pow",
    "print", "property", "range", "repr", "reversed", "round", "set", "setattr",
    "slice", "sorted", "staticmethod", "str", "sum", "super", "tuple", "type",
    "vars", "zip",
];

pub struct NamingConvention {
    keywords: HashSet<String>,
    builtins: HashSet<String>,
}

impl NamingConvention {
    pub fn new() -> Self {
        Self {
            keywords: PYTHON_KEYWORDS.iter().map(|s| s.to_string()).collect(),
            builtins: PYTHON_BUILTINS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Format a class name for Python
    /// Examples:
    /// - "PetsCreateError" -> "PetsCreateError"
    /// - "input.Option" -> "InputOption"
    /// - "output.Behavior" -> "OutputBehavior"
    pub fn format_class_name(&self, name: &str) -> String {
        // Handle module-prefixed names (e.g., "input.Option", "output.Pet")
        if let Some(dot_pos) = name.find('.') {
            let (module, type_name) = name.split_at(dot_pos);
            let type_name = &type_name[1..]; // Skip the dot
            
            // Capitalize module prefix and concatenate
            let module_pascal = self.to_pascal_case(module);
            format!("{}{}", module_pascal, type_name)
        } else {
            // Simple type name - use as-is
            name.to_string()
        }
    }

    /// Format a variant class name for Python
    /// This is used for enum variant types that need to be generated as separate classes
    /// Examples:
    /// - ("PetsCreateError", "InvalidIdentity") -> "PetsCreateErrorInvalidIdentityVariant"
    /// - ("input.Option", "Some") -> "InputOptionSomeVariant"
    pub fn format_variant_class_name(&self, enum_name: &str, variant_name: &str) -> String {
        let base_name = self.format_class_name(enum_name);
        let variant_pascal = self.to_pascal_case(variant_name);
        format!("{}{}Variant", base_name, variant_pascal)
    }

    /// Format a type reference for use in type annotations
    /// This handles the same transformations as format_class_name but is semantically
    /// distinct for clarity. Type references and class definitions must match exactly.
    pub fn format_type_reference(&self, name: &str) -> String {
        self.format_class_name(name)
    }

    /// Format a field name for Python
    /// Handles keyword escaping and snake_case conversion
    pub fn format_field_name(&self, name: &str) -> String {
        let snake_name = self.to_snake_case(name);
        
        // Escape Python keywords and builtins
        if self.keywords.contains(&snake_name) || self.builtins.contains(&snake_name) {
            format!("{}_", snake_name)
        } else if snake_name.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            // Field names can't start with a digit
            format!("field_{}", snake_name)
        } else {
            snake_name
        }
    }

    /// Format a method name for Python
    pub fn format_method_name(&self, name: &str) -> String {
        self.format_field_name(name) // Methods follow same rules as fields
    }

    /// Format a parameter name for Python
    pub fn format_parameter_name(&self, name: &str) -> String {
        self.format_field_name(name) // Parameters follow same rules as fields
    }

    /// Convert a string to PascalCase
    fn to_pascal_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = true;
        
        for c in s.chars() {
            if c == '_' || c == '-' || c == '.' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c);
            }
        }
        
        result
    }

    /// Convert a string to snake_case
    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut prev_was_upper = false;
        
        for (i, c) in s.chars().enumerate() {
            if c == '-' || c == '.' || c == ' ' {
                if !result.is_empty() && !result.ends_with('_') {
                    result.push('_');
                }
            } else if c.is_uppercase() && i > 0 {
                // Add underscore before uppercase letter if:
                // - Previous char was lowercase, or
                // - This starts a new word in acronym (e.g., "HTTPSConnection" -> "https_connection")
                let prev_char = s.chars().nth(i - 1);
                if let Some(prev) = prev_char {
                    if prev.is_lowercase() || (prev_was_upper && s.chars().nth(i + 1).map_or(false, |next| next.is_lowercase())) {
                        if !result.ends_with('_') {
                            result.push('_');
                        }
                    }
                }
                result.push(c.to_lowercase().next().unwrap_or(c));
                prev_was_upper = true;
            } else {
                result.push(c.to_lowercase().next().unwrap_or(c));
                prev_was_upper = false;
            }
        }
        
        result
    }
}

impl Default for NamingConvention {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_class_name() {
        let naming = NamingConvention::new();
        
        // Simple class names remain unchanged
        assert_eq!(naming.format_class_name("PetsCreateError"), "PetsCreateError");
        assert_eq!(naming.format_class_name("SimpleModel"), "SimpleModel");
        
        // Module-prefixed names get transformed
        assert_eq!(naming.format_class_name("input.Option"), "InputOption");
        assert_eq!(naming.format_class_name("output.Behavior"), "OutputBehavior");
        assert_eq!(naming.format_class_name("output.Pet"), "OutputPet");
    }

    #[test]
    fn test_format_variant_class_name() {
        let naming = NamingConvention::new();
        
        // Simple enum variants
        assert_eq!(
            naming.format_variant_class_name("PetsCreateError", "InvalidIdentity"),
            "PetsCreateErrorInvalidIdentityVariant"
        );
        
        // Module-prefixed enum variants
        assert_eq!(
            naming.format_variant_class_name("input.Option", "Some"),
            "InputOptionSomeVariant"
        );
        assert_eq!(
            naming.format_variant_class_name("output.Behavior", "Aggressive"),
            "OutputBehaviorAggressiveVariant"
        );
    }

    #[test]
    fn test_format_field_name() {
        let naming = NamingConvention::new();
        
        // Normal field names
        assert_eq!(naming.format_field_name("userName"), "user_name");
        assert_eq!(naming.format_field_name("HTTPSConnection"), "https_connection");
        
        // Python keywords get escaped
        assert_eq!(naming.format_field_name("class"), "class_");
        assert_eq!(naming.format_field_name("return"), "return_");
        
        // Fields starting with digits
        assert_eq!(naming.format_field_name("123field"), "field_123field");
    }

    #[test]
    fn test_naming_consistency() {
        let naming = NamingConvention::new();
        
        // Critical test: Class definitions and type references must match
        let enum_name = "input.Option";
        let variant_name = "Some";
        
        let class_def = naming.format_variant_class_name(enum_name, variant_name);
        let type_ref = naming.format_type_reference(&class_def);
        
        assert_eq!(class_def, type_ref, "Class definition and type reference must match!");
        assert_eq!(class_def, "InputOptionSomeVariant");
    }
}