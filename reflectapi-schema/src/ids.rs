/// Compiler-owned ID assignment for schema symbols.
///
/// SymbolIds are a compiler concept used during normalization and codegen.
/// They are NOT stored on raw schema types — raw types are the interchange
/// format (JSON-serializable, derive-macro-produced). Instead, IDs are
/// assigned here and passed to the normalizer as a side table.
use crate::symbol::STDLIB_TYPES;
use crate::{Schema, SymbolId, SymbolKind, Type, Typespace};
use std::collections::HashMap;

/// Side table of compiler-assigned SymbolIds, keyed by fully-qualified name.
///
/// Built once from a raw `Schema` and consumed by the normalizer to assign
/// stable identities when constructing `SemanticSchema`.
#[derive(Debug, Clone)]
pub struct SchemaIds {
    /// Schema root ID
    pub schema_id: SymbolId,
    /// Function name → SymbolId
    pub functions: HashMap<String, SymbolId>,
    /// Type FQN → SymbolId (includes types from both input and output typespaces)
    pub types: HashMap<String, SymbolId>,
    /// (parent FQN, member name) → SymbolId for fields and variants
    pub members: HashMap<(String, String), SymbolId>,
}

/// Build a `SchemaIds` side table from a raw schema.
///
/// Types that share a fully-qualified name across input and output typespaces
/// receive distinct SymbolIds via the disambiguator field.
pub fn build_schema_ids(schema: &Schema) -> SchemaIds {
    let mut ids = SchemaIds {
        schema_id: SymbolId::new(
            SymbolKind::Schema,
            vec!["__schema__".to_string(), schema.name.clone()],
        ),
        functions: HashMap::new(),
        types: HashMap::new(),
        members: HashMap::new(),
    };

    // Pre-register well-known stdlib types
    for &(name, kind) in STDLIB_TYPES {
        ids.types
            .entry(name.to_string())
            .or_insert_with(|| SymbolId::new(kind, split_path(name)));
    }

    // Register functions
    for function in &schema.functions {
        ids.functions
            .entry(function.name.clone())
            .or_insert_with(|| SymbolId::new(SymbolKind::Endpoint, vec![function.name.clone()]));
    }

    // Register types from both typespaces
    let stdlib_snapshot = ids.types.clone();
    let mut input_seen = stdlib_snapshot.clone();
    let mut output_seen = stdlib_snapshot;

    register_typespace_ids(&schema.input_types, &mut input_seen, &mut ids.members);
    register_typespace_ids(&schema.output_types, &mut output_seen, &mut ids.members);

    // For any FQN that appears in both typespaces with different types,
    // disambiguate the output typespace's IDs
    for (fqn, input_id) in &input_seen {
        if let Some(output_id) = output_seen.get(fqn) {
            if input_id == output_id {
                let input_ty = schema.input_types.get_type(fqn);
                let output_ty = schema.output_types.get_type(fqn);
                if let (Some(input_ty), Some(output_ty)) = (input_ty, output_ty) {
                    if input_ty != output_ty {
                        let disambiguated =
                            SymbolId::with_disambiguator(output_id.kind, output_id.path.clone(), 1);
                        // Re-register output type's members with the new parent ID
                        if let Some(output_ty) = schema.output_types.get_type(fqn) {
                            reregister_members(output_ty, &disambiguated, &mut ids.members);
                        }
                        output_seen.insert(fqn.clone(), disambiguated);
                    }
                }
            }
        }
    }

    // Merge both typespace maps (output_seen may have disambiguated entries)
    ids.types.extend(input_seen);
    for (fqn, output_id) in output_seen {
        // Output overwrites only if it has a disambiguated ID
        if output_id.disambiguator > 0 {
            // Store disambiguated IDs with a prefix to avoid overwriting input IDs
            ids.types.insert(format!("__output__::{fqn}"), output_id);
        } else {
            ids.types.entry(fqn).or_insert(output_id);
        }
    }

    ids
}

/// Register types from a typespace into the ID maps
fn register_typespace_ids(
    typespace: &Typespace,
    seen: &mut HashMap<String, SymbolId>,
    members: &mut HashMap<(String, String), SymbolId>,
) {
    for ty in typespace.types() {
        let type_name = ty.name().to_string();
        let id = seen
            .entry(type_name.clone())
            .or_insert_with(|| {
                let kind = match ty {
                    Type::Primitive(_) => SymbolKind::Primitive,
                    Type::Struct(_) => SymbolKind::Struct,
                    Type::Enum(_) => SymbolKind::Enum,
                };
                SymbolId::new(kind, split_path(&type_name))
            })
            .clone();

        register_type_members(ty, &id, members);
    }
}

/// Register field and variant IDs for a type
fn register_type_members(
    ty: &Type,
    parent_id: &SymbolId,
    members: &mut HashMap<(String, String), SymbolId>,
) {
    let parent_fqn = parent_id.qualified_name();
    match ty {
        Type::Primitive(_) => {}
        Type::Struct(s) => {
            register_struct_members(s, &parent_fqn, parent_id, members);
        }
        Type::Enum(e) => {
            register_enum_members(e, &parent_fqn, parent_id, members);
        }
    }
}

/// Register struct field IDs
fn register_struct_members(
    s: &crate::Struct,
    parent_fqn: &str,
    parent_id: &SymbolId,
    members: &mut HashMap<(String, String), SymbolId>,
) {
    match &s.fields {
        crate::Fields::Named(fields) => {
            for field in fields {
                let mut path = parent_id.path.clone();
                path.push(field.name.clone());
                members
                    .entry((parent_fqn.to_string(), field.name.clone()))
                    .or_insert_with(|| SymbolId::new(SymbolKind::Field, path));
            }
        }
        crate::Fields::Unnamed(fields) => {
            for (i, _field) in fields.iter().enumerate() {
                let arg_name = format!("arg{i:02}");
                let mut path = parent_id.path.clone();
                path.push(arg_name.clone());
                members
                    .entry((parent_fqn.to_string(), arg_name))
                    .or_insert_with(|| SymbolId::new(SymbolKind::Field, path));
            }
        }
        crate::Fields::None => {}
    }
}

/// Register enum variant and variant field IDs
fn register_enum_members(
    e: &crate::Enum,
    parent_fqn: &str,
    parent_id: &SymbolId,
    members: &mut HashMap<(String, String), SymbolId>,
) {
    for variant in &e.variants {
        let mut variant_path = parent_id.path.clone();
        variant_path.push(variant.name.clone());
        let variant_id = members
            .entry((parent_fqn.to_string(), variant.name.clone()))
            .or_insert_with(|| SymbolId::new(SymbolKind::Variant, variant_path.clone()))
            .clone();

        match &variant.fields {
            crate::Fields::Named(fields) => {
                for field in fields {
                    let mut field_path = variant_id.path.clone();
                    field_path.push(field.name.clone());
                    let variant_fqn = variant_id.qualified_name();
                    members
                        .entry((variant_fqn, field.name.clone()))
                        .or_insert_with(|| SymbolId::new(SymbolKind::Field, field_path));
                }
            }
            crate::Fields::Unnamed(fields) => {
                for (i, _field) in fields.iter().enumerate() {
                    let arg_name = format!("arg{i:02}");
                    let mut field_path = variant_id.path.clone();
                    field_path.push(arg_name.clone());
                    let variant_fqn = variant_id.qualified_name();
                    members
                        .entry((variant_fqn, arg_name))
                        .or_insert_with(|| SymbolId::new(SymbolKind::Field, field_path));
                }
            }
            crate::Fields::None => {}
        }
    }
}

/// Re-register members with a new parent ID (used after disambiguation)
fn reregister_members(
    ty: &Type,
    new_parent_id: &SymbolId,
    members: &mut HashMap<(String, String), SymbolId>,
) {
    let parent_fqn = new_parent_id.qualified_name();
    match ty {
        Type::Primitive(_) => {}
        Type::Struct(s) => {
            register_struct_members(s, &parent_fqn, new_parent_id, members);
        }
        Type::Enum(e) => {
            register_enum_members(e, &parent_fqn, new_parent_id, members);
        }
    }
}

/// Split a fully-qualified name into path components
fn split_path(fqn: &str) -> Vec<String> {
    fqn.split("::").map(|s| s.to_string()).collect()
}

impl SchemaIds {
    /// Look up the ID for a type by its FQN
    pub fn type_id(&self, fqn: &str) -> SymbolId {
        self.types
            .get(fqn)
            .cloned()
            .unwrap_or_else(|| SymbolId::new(infer_kind(fqn), split_path(fqn)))
    }

    /// Look up the ID for a member (field or variant) by parent FQN and member name
    pub fn member_id(&self, parent_fqn: &str, member_name: &str) -> SymbolId {
        self.members
            .get(&(parent_fqn.to_string(), member_name.to_string()))
            .cloned()
            .unwrap_or_else(|| {
                let mut path = split_path(parent_fqn);
                path.push(member_name.to_string());
                SymbolId::new(SymbolKind::Field, path)
            })
    }
}

fn infer_kind(fqn: &str) -> SymbolKind {
    // Default to Struct for unknown types — the normalizer will
    // assign the correct kind when it discovers the actual type
    if fqn.starts_with("std::") || fqn.starts_with("chrono::") || fqn.starts_with("uuid::") {
        SymbolKind::Primitive
    } else {
        SymbolKind::Struct
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field, Fields, Variant};

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
    fn test_build_schema_ids_basic() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        let mut s = crate::Struct::new("api::User");
        s.fields = Fields::Named(vec![Field::new("name".into(), "String".into())]);
        schema.input_types.insert_type(s.into());

        let ids = build_schema_ids(&schema);

        // Schema root ID should exist
        assert_eq!(ids.schema_id.kind, SymbolKind::Schema);
        assert!(ids.schema_id.path.contains(&"__schema__".to_string()));

        // Type ID should exist
        let user_id = ids.type_id("api::User");
        assert_eq!(user_id.kind, SymbolKind::Struct);
        assert_eq!(user_id.path, vec!["api", "User"]);

        // Field ID should exist
        let field_id = ids.member_id("api::User", "name");
        assert_eq!(field_id.kind, SymbolKind::Field);
        assert_eq!(field_id.path, vec!["api", "User", "name"]);
    }

    #[test]
    fn test_disambiguated_types() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        // Input "Foo" has field "a"
        let mut input_foo = crate::Struct::new("Foo");
        input_foo.fields = Fields::Named(vec![Field::new("a".into(), "u32".into())]);
        schema.input_types.insert_type(input_foo.into());

        // Output "Foo" has field "b" (different structure)
        let mut output_foo = crate::Struct::new("Foo");
        output_foo.fields = Fields::Named(vec![Field::new("b".into(), "u64".into())]);
        schema.output_types.insert_type(output_foo.into());

        let ids = build_schema_ids(&schema);

        // Output Foo should be disambiguated
        let output_id = ids
            .types
            .get("__output__::Foo")
            .expect("disambiguated output type should exist");
        assert_eq!(output_id.disambiguator, 1);
    }

    #[test]
    fn test_enum_member_ids() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        let mut e = crate::Enum::new("api::Status".into());
        e.variants = vec![Variant::new("Active".into())];
        schema.input_types.insert_type(e.into());

        let ids = build_schema_ids(&schema);

        let enum_id = ids.type_id("api::Status");
        assert_eq!(enum_id.kind, SymbolKind::Enum);

        let variant_id = ids.member_id("api::Status", "Active");
        assert_eq!(variant_id.kind, SymbolKind::Variant);
        assert_eq!(variant_id.path, vec!["api", "Status", "Active"]);
    }

    #[test]
    fn test_zero_padded_tuple_field_ordering() {
        let mut schema = Schema::new();
        schema.name = "Test".to_string();

        let mut tuple_struct = crate::Struct::new("BigTuple");
        let fields: Vec<Field> = (0..12)
            .map(|i| Field::new(format!("{i}"), format!("u{}", 8 + i).into()))
            .collect();
        tuple_struct.fields = Fields::Unnamed(fields);
        schema.input_types.insert_type(tuple_struct.into());

        let ids = build_schema_ids(&schema);

        // Verify zero-padded format
        let field0 = ids.member_id("BigTuple", "arg00");
        assert_eq!(field0.path.last().unwrap(), "arg00");

        let field10 = ids.member_id("BigTuple", "arg10");
        assert_eq!(field10.path.last().unwrap(), "arg10");
    }

    #[test]
    fn test_schema_root_id_does_not_collide_with_type() {
        let mut schema = Schema::new();
        schema.name = "User".to_string();

        let user_struct = crate::Struct::new("User");
        schema.input_types.insert_type(user_struct.into());

        let ids = build_schema_ids(&schema);

        let struct_id = ids.type_id("User");
        assert_ne!(
            ids.schema_id, struct_id,
            "Schema root ID should not collide with struct ID"
        );
        assert!(ids.schema_id.path.contains(&"__schema__".to_string()));
    }
}
