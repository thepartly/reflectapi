/// ID assignment utilities for ensuring unique, stable SymbolIds before normalization
///
/// This module fixes the core issue where JSON-deserialized schemas have default
/// SymbolIds that cause conflicts during normalization discovery.
use crate::{Enum, Schema, Struct, SymbolId, SymbolKind, Type, Typespace};
use std::collections::HashMap;

/// Ensure all symbols in the schema have unique, stable IDs based on their canonical names
pub fn ensure_symbol_ids(schema: &mut Schema) {
    // Assign schema ID if unknown
    if schema.id.is_unknown() {
        schema.id = SymbolId::new(SymbolKind::Struct, vec![schema.name.clone()]);
    }

    // Map canonical name -> assigned SymbolId (to keep input/output in sync)
    let mut seen: HashMap<String, SymbolId> = HashMap::new();

    // Pre-register common stdlib types
    register_stdlib_types(&mut seen);

    // Assign function IDs
    for f in &mut schema.functions {
        if f.id.is_unknown() {
            f.id = SymbolId::new(SymbolKind::Endpoint, vec![f.name.clone()]);
        }
    }

    // Assign type IDs in both typespaces
    assign_typespace_ids(&mut schema.input_types, &mut seen);
    assign_typespace_ids(&mut schema.output_types, &mut seen);
}

/// Pre-register well-known stdlib types that might be referenced but not always defined
fn register_stdlib_types(seen: &mut HashMap<String, SymbolId>) {
    let stdlib_types = [
        ("std::option::Option", SymbolKind::Enum),
        ("std::vec::Vec", SymbolKind::Primitive),
        ("std::collections::HashMap", SymbolKind::Primitive),
        ("std::collections::BTreeMap", SymbolKind::Primitive),
        ("std::string::String", SymbolKind::Primitive),
        ("std::tuple::Tuple0", SymbolKind::Primitive),
        ("i32", SymbolKind::Primitive),
        ("u32", SymbolKind::Primitive),
        ("i64", SymbolKind::Primitive),
        ("u64", SymbolKind::Primitive),
        ("f32", SymbolKind::Primitive),
        ("f64", SymbolKind::Primitive),
        ("bool", SymbolKind::Primitive),
        ("u8", SymbolKind::Primitive),
        ("i8", SymbolKind::Primitive),
        ("chrono::Utc", SymbolKind::Primitive),
        ("chrono::FixedOffset", SymbolKind::Primitive),
        ("chrono::DateTime", SymbolKind::Primitive),
        ("uuid::Uuid", SymbolKind::Primitive),
        ("url::Url", SymbolKind::Primitive),
        ("serde_json::Value", SymbolKind::Primitive),
    ];

    for (name, kind) in stdlib_types {
        seen.entry(name.to_string())
            .or_insert_with(|| SymbolId::new(kind, split_path(name)));
    }
}

/// Assign IDs to all types in a typespace
fn assign_typespace_ids(typespace: &mut Typespace, seen: &mut HashMap<String, SymbolId>) {
    // Create a new typespace with updated types
    let mut new_typespace = Typespace::new();

    // Process each type individually
    for ty in typespace.types() {
        let mut updated_type = ty.clone();
        let type_name = updated_type.name().to_string();
        assign_type_id(&type_name, &mut updated_type, seen);
        new_typespace.insert_type(updated_type);
    }

    // Replace the original typespace
    *typespace = new_typespace;
}

/// Assign a unique ID to a type and its nested members
fn assign_type_id(fqn: &str, ty: &mut Type, seen: &mut HashMap<String, SymbolId>) {
    // Get or create ID for this type
    let id = seen
        .entry(fqn.to_string())
        .or_insert_with(|| {
            let kind = match ty {
                Type::Primitive(_) => SymbolKind::Primitive,
                Type::Struct(_) => SymbolKind::Struct,
                Type::Enum(_) => SymbolKind::Enum,
            };
            SymbolId::new(kind, split_path(fqn))
        })
        .clone();

    // Assign ID to the type itself and its members
    match ty {
        Type::Primitive(p) => {
            if p.id.is_unknown() {
                p.id = id;
            }
        }
        Type::Struct(s) => {
            if s.id.is_unknown() {
                s.id = id.clone();
            }
            assign_struct_member_ids(s, &id);
        }
        Type::Enum(e) => {
            if e.id.is_unknown() {
                e.id = id.clone();
            }
            assign_enum_member_ids(e, &id);
        }
    }
}

/// Assign IDs to struct fields
fn assign_struct_member_ids(s: &mut Struct, owner: &SymbolId) {
    // We need to access fields mutably
    // Since we can't easily get mutable access through the existing API,
    // we'll work with the fields directly
    match &mut s.fields {
        crate::Fields::Named(fields) => {
            for field in fields {
                if field.id.is_unknown() {
                    let mut path = owner.path.clone();
                    path.push(field.name.clone());
                    field.id = SymbolId::new(SymbolKind::Field, path);
                }
            }
        }
        crate::Fields::Unnamed(fields) => {
            for (i, field) in fields.iter_mut().enumerate() {
                if field.id.is_unknown() {
                    let mut path = owner.path.clone();
                    path.push(format!("arg{}", i));
                    field.id = SymbolId::new(SymbolKind::Field, path);
                }
            }
        }
        crate::Fields::None => {}
    }
}

/// Assign IDs to enum variants and their fields
fn assign_enum_member_ids(e: &mut Enum, owner: &SymbolId) {
    for variant in &mut e.variants {
        if variant.id.is_unknown() {
            let mut path = owner.path.clone();
            path.push(variant.name.clone());
            variant.id = SymbolId::new(SymbolKind::Variant, path);
        }

        // Assign field IDs within the variant
        match &mut variant.fields {
            crate::Fields::Named(fields) => {
                for field in fields {
                    if field.id.is_unknown() {
                        let mut path = variant.id.path.clone();
                        path.push(field.name.clone());
                        field.id = SymbolId::new(SymbolKind::Field, path);
                    }
                }
            }
            crate::Fields::Unnamed(fields) => {
                for (i, field) in fields.iter_mut().enumerate() {
                    if field.id.is_unknown() {
                        let mut path = variant.id.path.clone();
                        path.push(format!("arg{}", i));
                        field.id = SymbolId::new(SymbolKind::Field, path);
                    }
                }
            }
            crate::Fields::None => {}
        }
    }
}

/// Split a fully-qualified name into path components
fn split_path(fqn: &str) -> Vec<String> {
    // For Rust-style names like myapi::proto::Type
    fqn.split("::").map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_path() {
        assert_eq!(
            split_path("std::option::Option"),
            vec!["std", "option", "Option"]
        );
        assert_eq!(
            split_path("myapi::proto::Headers"),
            vec!["myapi", "proto", "Headers"]
        );
        assert_eq!(split_path("SimpleType"), vec!["SimpleType"]);
    }

    #[test]
    fn test_is_unknown() {
        let unknown = SymbolId::default();
        assert!(unknown.is_unknown());

        let known = SymbolId::new(SymbolKind::Struct, vec!["MyType".to_string()]);
        assert!(!known.is_unknown());
    }
}
