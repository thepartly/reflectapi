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
        schema.id = SymbolId::new(
            SymbolKind::Schema,
            vec!["__schema__".to_string(), schema.name.clone()],
        );
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
            assign_struct_member_ids(s, &s.id.clone());
        }
        Type::Enum(e) => {
            if e.id.is_unknown() {
                e.id = id.clone();
            }
            assign_enum_member_ids(e, &e.id.clone());
        }
    }
}

/// Reassign the top-level ID of a type in a typespace (for disambiguation)
fn assign_disambiguated_id(typespace: &mut Typespace, fqn: &str, new_id: &SymbolId) {
    let types: Vec<_> = typespace.types().cloned().collect();
    let mut new_typespace = Typespace::new();
    for mut ty in types {
        if ty.name() == fqn {
            match &mut ty {
                Type::Primitive(p) => p.id = new_id.clone(),
                Type::Struct(s) => {
                    s.id = new_id.clone();
                    // Clear member IDs so they get re-assigned with the new parent
                    clear_struct_member_ids(s);
                    assign_struct_member_ids(s, new_id);
                }
                Type::Enum(e) => {
                    e.id = new_id.clone();
                    clear_enum_member_ids(e);
                    assign_enum_member_ids(e, new_id);
                }
            }
        }
        new_typespace.insert_type(ty);
    }
    *typespace = new_typespace;
}

fn clear_struct_member_ids(s: &mut Struct) {
    match &mut s.fields {
        crate::Fields::Named(fields) | crate::Fields::Unnamed(fields) => {
            for field in fields {
                field.id = SymbolId::default();
            }
        }
        crate::Fields::None => {}
    }
}

fn clear_enum_member_ids(e: &mut Enum) {
    for variant in &mut e.variants {
        variant.id = SymbolId::default();
        match &mut variant.fields {
            crate::Fields::Named(fields) | crate::Fields::Unnamed(fields) => {
                for field in fields {
                    field.id = SymbolId::default();
                }
            }
            crate::Fields::None => {}
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
                    path.push(format!("arg{i:02}"));
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
                        path.push(format!("arg{i:02}"));
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

    #[test]
    fn test_pre_assigned_id_member_paths_consistent() {
        // Regression: when a struct has a pre-assigned ID (e.g. from Struct::new),
        // field IDs must use the struct's actual ID as their parent, not the
        // seen-map ID which uses split_path and produces different path segments.
        use crate::{Field, Fields};

        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        // Struct::new("api::User") sets id.path = ["api::User"] (single unsplit element)
        let mut s = crate::Struct::new("api::User");
        s.fields = Fields::Named(vec![Field::new("name".into(), "String".into())]);
        schema.input_types.insert_type(s.into());

        ensure_symbol_ids(&mut schema);

        let s = schema
            .input_types
            .get_type("api::User")
            .unwrap()
            .as_struct()
            .unwrap();

        // The struct's ID path should be preserved as-is
        let struct_path = &s.id.path;

        // The field's path should start with the struct's actual path
        let field = s.fields().next().unwrap();
        let field_path = &field.id.path;

        assert_eq!(
            &field_path[..field_path.len() - 1],
            struct_path.as_slice(),
            "Field path prefix {field_path:?} should match struct path {struct_path:?}"
        );
    }

    #[test]
    fn test_pre_assigned_id_enum_member_paths_consistent() {
        // Same regression test for enums
        use crate::Variant;

        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        let mut e = crate::Enum::new("api::Status".into());
        e.variants = vec![Variant::new("Active".into())];
        schema.input_types.insert_type(e.into());

        ensure_symbol_ids(&mut schema);

        let e = schema
            .input_types
            .get_type("api::Status")
            .unwrap()
            .as_enum()
            .unwrap();

        let enum_path = &e.id.path;
        let variant = &e.variants[0];
        let variant_path = &variant.id.path;

        assert_eq!(
            &variant_path[..variant_path.len() - 1],
            enum_path.as_slice(),
            "Variant path prefix {variant_path:?} should match enum path {enum_path:?}"
        );
    }

    #[test]
    fn test_zero_padded_tuple_field_ordering() {
        // Regression: unnamed (tuple) fields with >= 10 elements must use zero-padded
        // indices so that lexicographic sorting preserves declaration order.
        use crate::{Field, Fields};

        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        let mut tuple_struct = crate::Struct::new("BigTuple");
        // Create 12 unnamed fields
        let fields: Vec<Field> = (0..12)
            .map(|i| Field::new(format!("{i}"), format!("u{}", 8 + i).into()))
            .collect();
        tuple_struct.fields = Fields::Unnamed(fields);
        schema.input_types.insert_type(tuple_struct.into());

        ensure_symbol_ids(&mut schema);

        let s = schema
            .input_types
            .get_type("BigTuple")
            .unwrap()
            .as_struct()
            .unwrap();

        let field_ids: Vec<String> = s
            .fields()
            .map(|f| f.id.path.last().unwrap().clone())
            .collect();

        // Verify zero-padded format: arg00, arg01, ..., arg09, arg10, arg11
        assert_eq!(field_ids.len(), 12);
        assert_eq!(field_ids[0], "arg00");
        assert_eq!(field_ids[1], "arg01");
        assert_eq!(field_ids[2], "arg02");
        assert_eq!(field_ids[9], "arg09");
        assert_eq!(field_ids[10], "arg10");
        assert_eq!(field_ids[11], "arg11");

        // Verify lexicographic sort preserves declaration order:
        // arg02 < arg10 (not arg10 < arg2 which would happen without padding)
        let mut sorted_ids = field_ids.clone();
        sorted_ids.sort();
        assert_eq!(
            field_ids, sorted_ids,
            "Zero-padded field IDs should sort in declaration order"
        );
    }

    #[test]
    fn test_disambiguated_id_updates_member_ids() {
        // Regression: when a type in the output typespace is disambiguated (because
        // a type with the same name but different fields exists in the input typespace),
        // the field IDs of the disambiguated type must reflect the new parent ID.
        use crate::{Field, Fields};

        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        // Input "Foo" has field "a"
        let mut input_foo = crate::Struct::new("Foo");
        input_foo.fields = Fields::Named(vec![Field::new("a".into(), "u32".into())]);
        schema.input_types.insert_type(input_foo.into());

        // Output "Foo" has field "b" (different structure triggers disambiguation)
        let mut output_foo = crate::Struct::new("Foo");
        output_foo.fields = Fields::Named(vec![Field::new("b".into(), "u64".into())]);
        schema.output_types.insert_type(output_foo.into());

        ensure_symbol_ids(&mut schema);

        // The output Foo should have disambiguator=1
        let output_struct = schema
            .output_types
            .get_type("Foo")
            .unwrap()
            .as_struct()
            .unwrap();
        assert_eq!(
            output_struct.id.disambiguator, 1,
            "Output Foo should have disambiguator=1, got {:?}",
            output_struct.id
        );

        // The field "b" should have a path consistent with the disambiguated parent
        let field_b = output_struct.fields().next().unwrap();
        let field_path_prefix = &field_b.id.path[..field_b.id.path.len() - 1];
        assert_eq!(
            field_path_prefix,
            output_struct.id.path.as_slice(),
            "Field path prefix {:?} should match disambiguated parent path {:?}",
            field_b.id.path,
            output_struct.id.path
        );
    }

    #[test]
    fn test_schema_root_id_does_not_collide_with_type() {
        // Regression: a schema named "User" with a struct also named "User"
        // should not produce colliding IDs. The schema root uses the "__schema__"
        // sentinel in its path to avoid this.
        let mut schema = Schema::new();
        schema.name = "User".to_string();

        let user_struct = crate::Struct::new("User");
        schema.input_types.insert_type(user_struct.into());

        ensure_symbol_ids(&mut schema);

        let struct_type = schema
            .input_types
            .get_type("User")
            .unwrap()
            .as_struct()
            .unwrap();

        assert_ne!(
            schema.id, struct_type.id,
            "Schema root ID {:?} should not collide with struct ID {:?}",
            schema.id, struct_type.id
        );

        // Verify the schema root uses the __schema__ sentinel
        assert!(
            schema.id.path.contains(&"__schema__".to_string()),
            "Schema root ID path should contain '__schema__' sentinel, got {:?}",
            schema.id.path
        );
        assert_eq!(schema.id.kind, SymbolKind::Schema);
    }
}
