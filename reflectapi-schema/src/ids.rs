/// ID assignment utilities for ensuring unique, stable SymbolIds before normalization
///
/// This module fixes the core issue where JSON-deserialized schemas have default
/// SymbolIds that cause conflicts during normalization discovery.
use crate::symbol::STDLIB_TYPES;
use crate::{Enum, Schema, Struct, SymbolId, SymbolKind, Type, Typespace};
use std::collections::HashMap;

/// Ensure all symbols in the schema have unique, stable IDs based on their canonical names
pub fn ensure_symbol_ids(schema: &mut Schema) {
    if schema.id.is_unknown() {
        schema.id = SymbolId::new(SymbolKind::Struct, vec![schema.name.clone()]);
    }

    let mut seen: HashMap<String, SymbolId> = HashMap::new();
    register_stdlib_types(&mut seen);

    for function in &mut schema.functions {
        if function.id.is_unknown() {
            function.id = SymbolId::new(SymbolKind::Endpoint, vec![function.name.clone()]);
        }
    }

    assign_typespace_ids(&mut schema.input_types, &mut seen);
    assign_typespace_ids(&mut schema.output_types, &mut seen);
}

/// Pre-register well-known stdlib types that might be referenced but not always defined
fn register_stdlib_types(seen: &mut HashMap<String, SymbolId>) {
    for &(name, kind) in STDLIB_TYPES {
        seen.entry(name.to_string())
            .or_insert_with(|| SymbolId::new(kind, split_path(name)));
    }
}

/// Assign IDs to all types in a typespace
fn assign_typespace_ids(typespace: &mut Typespace, seen: &mut HashMap<String, SymbolId>) {
    let mut new_typespace = Typespace::new();

    for ty in typespace.types() {
        let mut updated_type = ty.clone();
        let type_name = updated_type.name().to_string();
        assign_type_id(&type_name, &mut updated_type, seen);
        new_typespace.insert_type(updated_type);
    }

    *typespace = new_typespace;
}

/// Assign a unique ID to a type and its nested members
fn assign_type_id(fqn: &str, ty: &mut Type, seen: &mut HashMap<String, SymbolId>) {
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
                    path.push(format!("arg{i}"));
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
                        path.push(format!("arg{i}"));
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
