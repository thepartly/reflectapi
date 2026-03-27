/// ID assignment utilities for ensuring unique, stable SymbolIds before normalization
///
/// This module fixes the core issue where JSON-deserialized schemas have default
/// SymbolIds that cause conflicts during normalization discovery.
use crate::symbol::STDLIB_TYPES;
use crate::{Enum, Schema, Struct, SymbolId, SymbolKind, Type, Typespace};
use std::collections::HashMap;

/// Ensure all symbols in the schema have unique, stable IDs based on their canonical names.
///
/// Types that share a fully-qualified name across input and output typespaces
/// receive distinct SymbolIds via the disambiguator field. This prevents
/// collisions when types with the same name have different definitions in
/// each typespace (e.g., request vs response variants of the same type).
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

    // Use separate seen maps per typespace, sharing stdlib registrations.
    // This ensures types with the same FQN in different typespaces get
    // distinct SymbolIds (via disambiguator) when they are different types.
    let mut input_seen = seen.clone();
    let mut output_seen = seen;
    assign_typespace_ids(&mut schema.input_types, &mut input_seen);
    assign_typespace_ids(&mut schema.output_types, &mut output_seen);

    // For any FQN that appears in both typespaces with different types,
    // disambiguate the output typespace's IDs
    for (fqn, input_id) in &input_seen {
        if let Some(output_id) = output_seen.get(fqn) {
            if input_id == output_id {
                // Same ID means same type — check if the actual types differ
                let input_ty = schema.input_types.get_type(fqn);
                let output_ty = schema.output_types.get_type(fqn);
                if let (Some(input_ty), Some(output_ty)) = (input_ty, output_ty) {
                    if input_ty != output_ty {
                        // Different types with same name — disambiguate output
                        let disambiguated =
                            SymbolId::with_disambiguator(output_id.kind, output_id.path.clone(), 1);
                        assign_disambiguated_id(&mut schema.output_types, fqn, &disambiguated);
                    }
                }
            }
        }
    }
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
            let owner = s.id.clone();
            assign_struct_member_ids(s, &owner);
        }
        Type::Enum(e) => {
            if e.id.is_unknown() {
                e.id = id.clone();
            }
            let owner = e.id.clone();
            assign_enum_member_ids(e, &owner);
        }
    }
}

/// Reassign the top-level ID of a type in a typespace (for disambiguation)
fn assign_disambiguated_id(typespace: &mut Typespace, fqn: &str, new_id: &SymbolId) {
    // Rebuild typespace with the updated ID
    let types: Vec<_> = typespace.types().cloned().collect();
    let mut new_typespace = Typespace::new();
    for mut ty in types {
        if ty.name() == fqn {
            match &mut ty {
                Type::Primitive(p) => p.id = new_id.clone(),
                Type::Struct(s) => s.id = new_id.clone(),
                Type::Enum(e) => e.id = new_id.clone(),
            }
        }
        new_typespace.insert_type(ty);
    }
    *typespace = new_typespace;
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
