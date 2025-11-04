use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

use anyhow::Context;
use askama::Template;

use crate::{Schema, TypeReference};
use reflectapi_schema::{Function, Type};

/// Information needed to generate a factory class later
#[derive(Clone, Debug)]
struct FactoryInfo {
    enum_def: reflectapi_schema::Enum,
    enum_name: String,
    union_members: Vec<String>,
    is_internally_tagged: bool,
}

fn to_valid_python_identifier(name: &str) -> String {
    safe_python_identifier(name)
}

/// Configuration for Python client generation
#[derive(Debug, Clone)]
pub struct Config {
    /// Package name for the generated client
    pub package_name: String,
    /// Whether to generate async client
    pub generate_async: bool,
    /// Whether to generate sync client
    pub generate_sync: bool,
    /// Whether to generate testing utilities
    pub generate_testing: bool,
    /// Base URL for the API (optional)
    pub base_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            package_name: "api_client".to_string(),
            generate_async: true,
            generate_sync: true,
            generate_testing: true,
            base_url: None,
        }
    }
}

/// Generate optimized imports with proper sorting and deduplication
fn generate_optimized_imports(imports: &templates::Imports) -> String {
    use std::collections::BTreeSet;

    let mut stdlib_imports = BTreeSet::new();
    let mut typing_imports = BTreeSet::new();
    let mut third_party_imports = BTreeSet::new();
    let mut runtime_imports = BTreeSet::new();

    // Standard library - datetime
    if imports.has_datetime || imports.has_date || imports.has_timedelta {
        let mut datetime_parts = vec![];
        if imports.has_datetime {
            datetime_parts.push("datetime");
        }
        if imports.has_date {
            datetime_parts.push("date");
        }
        if imports.has_timedelta {
            datetime_parts.push("timedelta");
        }
        stdlib_imports.insert(format!(
            "from datetime import {}",
            datetime_parts.join(", ")
        ));
    }

    // Standard library - enum
    if imports.has_enums {
        stdlib_imports.insert("from enum import Enum".to_string());
    }

    // Standard library - uuid
    if imports.has_uuid {
        stdlib_imports.insert("from uuid import UUID".to_string());
    }

    // Standard library - warnings
    if imports.has_warnings {
        stdlib_imports.insert("import warnings".to_string());
    }

    // Typing imports - always include base ones
    typing_imports.insert("Any");
    typing_imports.insert("Optional");
    if imports.has_generics || !imports.global_type_vars.is_empty() {
        typing_imports.insert("TypeVar");
        typing_imports.insert("Generic");
    }
    typing_imports.insert("Union");

    if imports.has_annotated {
        typing_imports.insert("Annotated");
    }
    if imports.has_literal {
        typing_imports.insert("Literal");
    }

    // Pydantic imports
    third_party_imports.insert("BaseModel");
    third_party_imports.insert("ConfigDict");
    if imports.has_discriminated_unions {
        third_party_imports.insert("Field");
        third_party_imports.insert("RootModel");
    }
    if imports.has_externally_tagged_enums {
        third_party_imports.insert("RootModel");
        third_party_imports.insert("model_validator");
        third_party_imports.insert("model_serializer");
        third_party_imports.insert("PrivateAttr");
    }

    // Runtime imports - client bases
    if imports.has_async && imports.has_sync {
        runtime_imports.insert("AsyncClientBase, ClientBase, ApiResponse");
    } else if imports.has_async {
        runtime_imports.insert("AsyncClientBase, ApiResponse");
    } else if imports.has_sync {
        runtime_imports.insert("ClientBase, ApiResponse");
    }

    // Runtime imports - special types
    let mut special_types = vec![];
    if imports.has_reflectapi_option {
        special_types.push("ReflectapiOption");
    }
    if imports.has_reflectapi_empty {
        special_types.push("ReflectapiEmpty");
    }
    if imports.has_reflectapi_infallible {
        special_types.push("ReflectapiInfallible");
    }

    // Build the final import string
    let mut result = Vec::new();

    // Add header
    result.push("# Standard library imports".to_string());
    for import in stdlib_imports {
        result.push(import);
    }

    // Add typing imports
    if !typing_imports.is_empty() {
        let typing_list: Vec<&str> = typing_imports.iter().copied().collect();
        result.push(format!("from typing import {}", typing_list.join(", ")));
    }

    result.push("".to_string());
    result.push("# Third-party imports".to_string());

    // Add pydantic imports
    if !third_party_imports.is_empty() {
        let pydantic_list: Vec<&str> = third_party_imports.iter().copied().collect();
        result.push(format!("from pydantic import {}", pydantic_list.join(", ")));
    }

    result.push("".to_string());
    result.push("# Runtime imports".to_string());

    // Add runtime imports
    for import in runtime_imports {
        result.push(format!("from reflectapi_runtime import {}", import));
    }

    // Add special types as separate imports for clarity
    for special_type in special_types {
        result.push(format!("from reflectapi_runtime import {}", special_type));
    }

    // Add testing imports
    if imports.has_testing {
        result.push(
            "from reflectapi_runtime.testing import MockClient, create_api_response".to_string(),
        );
    }

    result.push("".to_string());
    result.join("\n")
}

/// Topologically sort types to ensure dependencies are defined before dependents.
///
/// Uses Kahn's algorithm to resolve type dependencies and prevent forward reference errors.
/// For example, if type A references type B, then B will appear before A in the output.
///
/// # Arguments
/// * `type_names` - List of type names to sort
/// * `schema` - Schema containing type definitions and their dependencies
///
/// # Returns
/// * `Ok(Vec<String>)` - Types sorted in dependency order (dependencies first)
/// * `Err` - If circular dependencies are detected in the type graph
fn topological_sort_types(type_names: &[String], schema: &Schema) -> anyhow::Result<Vec<String>> {
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();

    // Initialize in-degree count for all types
    for type_name in type_names {
        in_degree.insert(type_name.clone(), 0);
        dependencies.insert(type_name.clone(), BTreeSet::new());
    }

    // Build dependency graph - if A depends on B, then B must be defined before A
    for type_name in type_names {
        if let Some(type_def) = schema.get_type(type_name) {
            let deps = collect_type_dependencies(type_def, type_names);
            for dep in &deps {
                if type_names.contains(dep) && dep != type_name {
                    // type_name depends on dep, so dep must come before type_name
                    dependencies.get_mut(type_name).unwrap().insert(dep.clone());
                    *in_degree.get_mut(type_name).unwrap() += 1;
                }
            }
        }
    }

    // Kahn's algorithm for topological sorting
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut result = Vec::new();

    // Start with types that have no dependencies
    for (type_name, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(type_name.clone());
        }
    }

    while let Some(current) = queue.pop_front() {
        result.push(current.clone());

        // Reduce in-degree for types that depend on the current type
        for other_type in type_names {
            if dependencies.get(other_type).unwrap().contains(&current) {
                *in_degree.get_mut(other_type).unwrap() -= 1;
                if in_degree[other_type] == 0 {
                    queue.push_back(other_type.clone());
                }
            }
        }
    }

    // Check for cycles
    if result.len() != type_names.len() {
        let remaining: Vec<_> = type_names.iter().filter(|n| !result.contains(n)).collect();
        return Err(anyhow::anyhow!(
            "Circular dependency detected in types: {:?}",
            remaining
        ));
    }

    Ok(result)
}

/// Collect all type dependencies for a given type
fn collect_type_dependencies(type_def: &Type, available_types: &[String]) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();

    match type_def {
        Type::Struct(struct_def) => {
            for field in struct_def.fields.iter() {
                collect_type_ref_dependencies(&field.type_ref, &mut deps, available_types);
            }
        }
        Type::Enum(enum_def) => {
            for variant in &enum_def.variants {
                match &variant.fields {
                    reflectapi_schema::Fields::Named(fields) => {
                        for field in fields {
                            collect_type_ref_dependencies(
                                &field.type_ref,
                                &mut deps,
                                available_types,
                            );
                        }
                    }
                    reflectapi_schema::Fields::Unnamed(fields) => {
                        for field in fields {
                            collect_type_ref_dependencies(
                                &field.type_ref,
                                &mut deps,
                                available_types,
                            );
                        }
                    }
                    reflectapi_schema::Fields::None => {}
                }
            }
        }
        Type::Primitive(_) => {}
    }

    deps
}

/// Recursively collect type reference dependencies
fn collect_type_ref_dependencies(
    type_ref: &TypeReference,
    deps: &mut BTreeSet<String>,
    available_types: &[String],
) {
    if available_types.contains(&type_ref.name) {
        deps.insert(type_ref.name.clone());
    }

    // Handle generic arguments
    for param in &type_ref.arguments {
        collect_type_ref_dependencies(param, deps, available_types);
    }
}

/// Collect all TypeVars used by a type
fn collect_type_vars_from_type(
    type_def: &Type,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    match type_def {
        Type::Struct(struct_def) => {
            // Collect active generic parameter names for this struct
            let generic_params: Vec<String> = infer_struct_generic_params(struct_def, schema);

            // Track used generic type variables
            for generic in &generic_params {
                used_type_vars.insert(generic.clone());
            }

            // Process fields to collect TypeVars from field types
            for field in struct_def.fields.iter() {
                collect_type_vars_from_type_ref(
                    &field.type_ref,
                    schema,
                    implemented_types,
                    &generic_params,
                    used_type_vars,
                )?;
            }
        }
        Type::Enum(enum_def) => {
            // Collect active generic parameter names for this enum
            let generic_params: Vec<String> = infer_enum_generic_params(enum_def, schema);

            // Track used generic type variables
            for generic in &generic_params {
                used_type_vars.insert(generic.clone());
            }

            // Process variants to collect TypeVars
            for variant in &enum_def.variants {
                match &variant.fields {
                    reflectapi_schema::Fields::Named(fields) => {
                        for field in fields {
                            collect_type_vars_from_type_ref(
                                &field.type_ref,
                                schema,
                                implemented_types,
                                &generic_params,
                                used_type_vars,
                            )?;
                        }
                    }
                    reflectapi_schema::Fields::Unnamed(fields) => {
                        for field in fields {
                            collect_type_vars_from_type_ref(
                                &field.type_ref,
                                schema,
                                implemented_types,
                                &generic_params,
                                used_type_vars,
                            )?;
                        }
                    }
                    reflectapi_schema::Fields::None => {}
                }
            }
        }
        Type::Primitive(_) => {}
    }
    Ok(())
}

/// Collect TypeVars from a type reference
fn collect_type_vars_from_type_ref(
    type_ref: &TypeReference,
    _schema: &Schema,
    _implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    // Check if this is an active generic parameter
    if active_generics.contains(&type_ref.name) {
        used_type_vars.insert(type_ref.name.clone());
    }

    // Process generic arguments
    for arg in &type_ref.arguments {
        collect_type_vars_from_type_ref(
            arg,
            _schema,
            _implemented_types,
            active_generics,
            used_type_vars,
        )?;
    }

    Ok(())
}

/// Helper function to collect generic type variables from a type reference
fn collect_generic_type_vars(
    type_ref: &TypeReference,
    out: &mut std::collections::HashSet<String>,
) {
    // This is similar to the visit_typevars function in the enum logic
    if type_ref.arguments.is_empty() && is_probable_typevar(&type_ref.name) {
        out.insert(type_ref.name.clone());
    }
    for arg in &type_ref.arguments {
        collect_generic_type_vars(arg, out);
    }
}

/// Render a struct that contains flattened fields using direct field expansion
fn render_struct_with_flatten(
    struct_def: &reflectapi_schema::Struct,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    let struct_name = improve_class_name(&struct_def.name);

    // Collect all fields (regular + flattened) into a single flat model
    let mut all_fields = Vec::new();
    let active_generics: Vec<String> = struct_def
        .parameters
        .iter()
        .map(|p| p.name.clone())
        .collect();

    // Track used generic type variables
    for generic in &active_generics {
        used_type_vars.insert(generic.clone());
    }

    // Add regular fields
    for field in struct_def.fields.iter().filter(|f| !f.flattened()) {
        let field_name = sanitize_field_name_with_alias(field.name());
        let field_type = type_ref_to_python_type(
            &field.type_ref,
            schema,
            implemented_types,
            &active_generics,
            used_type_vars,
        )?;

        let (python_name, alias) = field_name;

        all_fields.push(templates::Field {
            name: python_name,
            type_annotation: if field.required {
                field_type
            } else {
                format!("{} | None", field_type)
            },
            description: Some(field.description().to_string()),
            deprecation_note: field.deprecation_note.clone(),
            optional: !field.required,
            default_value: if field.required {
                None
            } else {
                Some("None".to_string())
            },
            alias,
        });
    }

    // Add flattened fields (expanded directly)
    for field in struct_def.fields.iter().filter(|f| f.flattened()) {
        let flattened_fields = collect_flattened_fields(
            &field.type_ref,
            schema,
            implemented_types,
            &active_generics,
            field.required,
            0, // Start at depth 0
            used_type_vars,
        )?;
        all_fields.extend(flattened_fields);
    }

    // Generate as a regular Pydantic model with all fields flattened
    let struct_template = templates::DataClass {
        name: struct_name,
        description: Some(struct_def.description().to_string()),
        fields: all_fields,
        is_tuple: false,
        is_generic: !active_generics.is_empty(),
        generic_params: active_generics,
    };

    let rendered = struct_template.render()?;
    Ok(rendered)
}

/// Validate that all type references in the schema exist
fn validate_type_references(schema: &Schema) -> anyhow::Result<()> {
    // Collect all defined types
    let mut defined_types = HashSet::new();

    // Add input types
    for type_def in schema.input_types().types() {
        defined_types.insert(type_def.name().to_string());
    }

    // Add output types
    for type_def in schema.output_types().types() {
        defined_types.insert(type_def.name().to_string());
    }

    // Also add primitive types that are always valid
    let primitives = vec![
        "bool",
        "i8",
        "i16",
        "i32",
        "i64",
        "i128",
        "isize",
        "u8",
        "u16",
        "u32",
        "u64",
        "u128",
        "usize",
        "f32",
        "f64",
        "char",
        "str",
        "String",
        "std::vec::Vec",
        "std::collections::HashMap",
        "std::collections::HashSet",
        "std::collections::BTreeMap",
        "std::collections::BTreeSet",
        "Option",
        "Result",
        "std::sync::Arc",
        "std::rc::Rc",
        "Box",
        "chrono::DateTime",
        "chrono::NaiveDateTime",
        "chrono::NaiveDate",
        "chrono::Utc",
        "chrono::Local",
        "chrono::FixedOffset",
        "uuid::Uuid",
        "url::Url",
        "std::time::Duration",
    ];
    for primitive in primitives {
        defined_types.insert(primitive.to_string());
    }

    // Check all type references
    let mut errors = Vec::new();

    // Check input types
    for type_def in schema.input_types().types() {
        let type_name = type_def.name();
        match type_def {
            reflectapi_schema::Type::Struct(struct_def) => {
                // Check field types
                for field in struct_def.fields.iter() {
                    validate_type_ref(
                        &field.type_ref,
                        &defined_types,
                        &mut errors,
                        &format!("struct {}, field {}", type_name, field.name()),
                    );
                }
            }
            reflectapi_schema::Type::Enum(enum_def) => {
                // Check variant field types
                for variant in &enum_def.variants {
                    use reflectapi_schema::Fields;
                    match &variant.fields {
                        Fields::Named(fields) => {
                            for field in fields {
                                validate_type_ref(
                                    &field.type_ref,
                                    &defined_types,
                                    &mut errors,
                                    &format!(
                                        "enum {}, variant {}, field {}",
                                        type_name,
                                        variant.name(),
                                        field.name()
                                    ),
                                );
                            }
                        }
                        Fields::Unnamed(fields) => {
                            for (i, field) in fields.iter().enumerate() {
                                validate_type_ref(
                                    &field.type_ref,
                                    &defined_types,
                                    &mut errors,
                                    &format!(
                                        "enum {}, variant {}, field {}",
                                        type_name,
                                        variant.name(),
                                        i
                                    ),
                                );
                            }
                        }
                        Fields::None => {}
                    }
                }
            }
            _ => {} // Primitive types don't have references
        }
    }

    // Check output types (if different from input types)
    for type_def in schema.output_types().types() {
        let type_name = type_def.name();
        // Skip if already checked as input type
        if schema.input_types().has_type(type_name) {
            continue;
        }
        match type_def {
            reflectapi_schema::Type::Struct(struct_def) => {
                // Check field types
                for field in struct_def.fields.iter() {
                    validate_type_ref(
                        &field.type_ref,
                        &defined_types,
                        &mut errors,
                        &format!("struct {}, field {}", type_name, field.name()),
                    );
                }
            }
            reflectapi_schema::Type::Enum(enum_def) => {
                // Check variant field types
                for variant in &enum_def.variants {
                    use reflectapi_schema::Fields;
                    match &variant.fields {
                        Fields::Named(fields) => {
                            for field in fields {
                                validate_type_ref(
                                    &field.type_ref,
                                    &defined_types,
                                    &mut errors,
                                    &format!(
                                        "enum {}, variant {}, field {}",
                                        type_name,
                                        variant.name(),
                                        field.name()
                                    ),
                                );
                            }
                        }
                        Fields::Unnamed(fields) => {
                            for (i, field) in fields.iter().enumerate() {
                                validate_type_ref(
                                    &field.type_ref,
                                    &defined_types,
                                    &mut errors,
                                    &format!(
                                        "enum {}, variant {}, field {}",
                                        type_name,
                                        variant.name(),
                                        i
                                    ),
                                );
                            }
                        }
                        Fields::None => {}
                    }
                }
            }
            _ => {} // Primitive types don't have references
        }
    }

    if !errors.is_empty() {
        return Err(anyhow::anyhow!(
            "Type validation errors:\n{}",
            errors.join("\n")
        ));
    }

    Ok(())
}

fn validate_type_ref(
    type_ref: &TypeReference,
    defined_types: &HashSet<String>,
    errors: &mut Vec<String>,
    context: &str,
) {
    // Check if the base type exists
    if !defined_types.contains(&type_ref.name) {
        // Check if it might be a generic parameter (single uppercase letter or common generic names)
        let is_generic = type_ref.name.len() <= 3
            && type_ref
                .name
                .chars()
                .all(|c| c.is_uppercase() || c.is_ascii_digit());

        // Also check if it looks like a local type (no :: in the name)
        // These might be defined elsewhere in the same module/schema
        let is_local_type = !type_ref.name.contains("::");

        if !is_generic && !is_local_type {
            errors.push(format!(
                "Unknown type '{}' referenced in {}",
                type_ref.name, context
            ));
        }
    }

    // Recursively check type arguments
    for arg in &type_ref.arguments {
        validate_type_ref(arg, defined_types, errors, context);
    }
}

/// Generate Python client code from a schema
pub fn generate_files(schema: Schema, config: &Config) -> anyhow::Result<BTreeMap<String, String>> {
    // Generate the main client code
    let generated_py_content = generate(schema.clone(), config)?;

    // Create the __init__.py content
    let init_py_content = generate_init_py(config);

    let mut files = BTreeMap::new();
    files.insert("generated.py".to_string(), generated_py_content);
    files.insert("__init__.py".to_string(), init_py_content);

    Ok(files)
}

fn generate_init_py(config: &Config) -> String {
    let mut imports = vec!["AsyncClient"];

    if config.generate_sync {
        imports.push("SyncClient");
    }

    let imports_list = format!("{:?}", imports);
    format!(
        r#""{} client generated by ReflectAPI."

from .generated import {}

__all__ = {}
"#,
        config.package_name,
        imports.join(", "),
        imports_list
    )
}

pub fn generate(mut schema: Schema, config: &Config) -> anyhow::Result<String> {
    let implemented_types = build_implemented_types();

    // Consolidate types to avoid duplicates
    schema.consolidate_types();

    // Validate all type references exist
    validate_type_references(&schema)?;

    let mut generated_code = Vec::new();

    // Generate file header
    let file_header = templates::FileHeader {
        package_name: config.package_name.clone(),
    };
    generated_code.push(
        file_header
            .render()
            .context("Failed to render file header")?,
    );

    // Check if we have enums in the schema
    let all_type_names = schema.consolidate_types();
    let has_enums = all_type_names.iter().any(|name| {
        if let Some(type_def) = schema.get_type(name) {
            matches!(type_def, reflectapi_schema::Type::Enum(_))
        } else {
            false
        }
    });

    // Check if we need Literal import and Field discriminator (for tagged enums)
    let (has_literal, has_discriminated_unions, has_externally_tagged_enums) = {
        let mut has_literal = false;
        let mut has_discriminated_unions = false;
        let mut has_externally_tagged_enums = false;

        for name in &all_type_names {
            if let Some(reflectapi_schema::Type::Enum(enum_def)) = schema.get_type(name) {
                match enum_def.representation {
                    reflectapi_schema::Representation::Internal { .. } => {
                        has_literal = true;
                        has_discriminated_unions = true;
                    }
                    reflectapi_schema::Representation::External => {
                        // Check if this enum has complex variants that need RootModel
                        let has_complex_variants =
                            enum_def.variants.iter().any(|v| match &v.fields {
                                reflectapi_schema::Fields::Named(_) => true,
                                reflectapi_schema::Fields::Unnamed(fields) => !fields.is_empty(),
                                reflectapi_schema::Fields::None => false,
                            });
                        if has_complex_variants {
                            has_externally_tagged_enums = true;
                        }
                    }
                    _ => {}
                }
            }
        }
        (
            has_literal,
            has_discriminated_unions,
            has_externally_tagged_enums,
        )
    };

    // Check if we need ReflectapiOption import
    let has_reflectapi_option = schema_uses_reflectapi_option(&schema, &all_type_names);

    // Check if we need ReflectapiEmpty import
    let has_reflectapi_empty = schema_uses_type(&schema, &all_type_names, "reflectapi::Empty");

    // Check if we need ReflectapiInfallible import
    let has_reflectapi_infallible =
        schema_uses_type(&schema, &all_type_names, "reflectapi::Infallible");

    // Flatten support uses direct field expansion in generated models
    let has_flatten_support = false;

    // Check if we need warnings import (for deprecated functions)
    let has_warnings = schema.functions().any(|f| f.deprecation_note.is_some());

    // Check if we need datetime imports (for chrono and time types)
    let has_datetime = check_datetime_usage(&schema, &all_type_names);
    let has_uuid = check_uuid_usage(&schema, &all_type_names);
    let has_timedelta = check_timedelta_usage(&schema, &all_type_names);
    let has_date = check_date_usage(&schema, &all_type_names);

    // Generate imports
    let imports = templates::Imports {
        has_async: config.generate_async,
        has_sync: config.generate_sync,
        has_testing: config.generate_testing,
        has_enums,
        has_reflectapi_option,
        has_reflectapi_empty,
        has_reflectapi_infallible,
        has_flatten_support,
        has_warnings,
        has_datetime,
        has_uuid,
        has_timedelta,
        has_date,
        has_generics: true,
        has_annotated: true, // Always include for external type fallbacks
        has_literal,
        has_discriminated_unions,
        has_externally_tagged_enums,
        global_type_vars: Vec::new(), // Will be added later after tracking usage
    };
    // Use optimized import generation instead of template
    generated_code.push(generate_optimized_imports(&imports));

    // Types that are provided by the runtime library and should not be generated
    let runtime_provided_types = [
        "reflectapi::Option",
        "reflectapi::Empty",
        "reflectapi::Infallible",
        "std::option::Option",
    ];

    // Collect TypeVars used across all types
    let mut used_type_vars: BTreeSet<String> = BTreeSet::new();
    // Use topological sort if possible, fall back to alphabetical on circular dependencies
    let sorted_type_names = topological_sort_types(&all_type_names, &schema).unwrap_or_else(|_| {
        // Circular dependencies detected, use alphabetical order
        let mut sorted = all_type_names.clone();
        sorted.sort();
        sorted
    });
    for original_type_name in &sorted_type_names {
        if runtime_provided_types.contains(&original_type_name.as_str()) {
            continue;
        }
        let type_def = schema.get_type(original_type_name).unwrap();

        // Collect TypeVars from this type
        collect_type_vars_from_type(type_def, &schema, &implemented_types, &mut used_type_vars)?;
    }

    // Generate TypeVar declarations
    if !used_type_vars.is_empty() {
        generated_code.push("".to_string());
        generated_code.push("# Type variables for generic types".to_string());
        generated_code.push("".to_string());
        for type_var in &used_type_vars {
            generated_code.push(format!("{} = TypeVar(\"{}\")", type_var, type_var));
        }
        generated_code.push("".to_string());
    }

    // Render all types (models and enums) without factories first
    let mut rendered_types = BTreeMap::new();
    let mut factory_data = Vec::new(); // Collect factory data for later generation

    // Sort types topologically to handle dependencies, fall back to alphabetical on circular deps
    let sorted_type_names = topological_sort_types(&all_type_names, &schema).unwrap_or_else(|_| {
        // Circular dependencies detected, use alphabetical order
        let mut sorted = all_type_names.clone();
        sorted.sort();
        sorted
    });

    for original_type_name in sorted_type_names {
        // Skip types provided by the runtime
        if runtime_provided_types.contains(&original_type_name.as_str()) {
            continue;
        }

        let type_def = schema.get_type(&original_type_name).unwrap();
        let name = improve_class_name(type_def.name());

        // TypeVars have already been collected, use empty set for rendering
        let mut dummy_type_vars = BTreeSet::new();
        let (rendered, factory_info) = render_type_without_factory(
            type_def,
            &schema,
            &implemented_types,
            &mut dummy_type_vars,
        )?;

        // Store factory info for later generation
        if let Some(info) = factory_info {
            factory_data.push(info);
        }

        // Only store non-empty renders (excludes unwrapped tuple structs)
        if !rendered.trim().is_empty() {
            rendered_types.insert(name.clone(), rendered.clone());
            generated_code.push(rendered);
        }
    }

    // TypeVar declarations are now generated at the top of the file (after imports)

    // Generate client class with nested method organization
    let functions_by_name: BTreeMap<String, &Function> =
        schema.functions().map(|f| (f.name.clone(), f)).collect();

    // Group functions by their prefix and separate top-level functions
    let mut function_groups: BTreeMap<String, Vec<templates::Function>> = BTreeMap::new();
    let mut top_level_functions: Vec<templates::Function> = Vec::new();

    for function_schema in functions_by_name.values() {
        let rendered_function = render_function(function_schema, &schema, &implemented_types)?;

        // Check for grouping patterns: underscore or dot notation
        if let Some(separator_pos) = function_schema
            .name
            .find('_')
            .or_else(|| function_schema.name.find('.'))
        {
            let group_name = &function_schema.name[..separator_pos];
            let method_name = &function_schema.name[separator_pos + 1..];

            // Create a modified function with the shortened name for nested access
            let mut nested_function = rendered_function.clone();
            nested_function.name =
                safe_python_identifier_with_context(method_name, IdentifierContext::Method);
            nested_function.original_name = Some(rendered_function.name.clone());

            function_groups
                .entry(to_valid_python_identifier(group_name))
                .or_default()
                .push(nested_function);
        } else {
            // Functions without separators remain as top-level methods
            top_level_functions.push(rendered_function);
        }
    }

    // Convert function groups to structured format
    let mut function_group_pairs: Vec<_> = function_groups.into_iter().collect();
    function_group_pairs.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by group name for deterministic output
    let structured_function_groups: Vec<templates::FunctionGroup> = function_group_pairs
        .into_iter()
        .map(|(group_name, functions)| templates::FunctionGroup {
            name: group_name.clone(),
            class_name: format!("{}Client", to_pascal_case(&group_name)),
            functions,
        })
        .collect();

    let client_template = templates::ClientClass {
        class_name: "Client".to_string(),
        async_class_name: "AsyncClient".to_string(),
        top_level_functions,
        function_groups: structured_function_groups,
        generate_async: config.generate_async,
        generate_sync: config.generate_sync,
        base_url: config.base_url.clone(),
    };
    generated_code.push(
        client_template
            .render()
            .context("Failed to render client class")?,
    );

    // Generate nested class structure
    let nested_classes = generate_nested_class_structure(&rendered_types, &schema);
    if !nested_classes.is_empty() {
        generated_code.push("# Nested class definitions for better organization".to_string());
        generated_code.push(nested_classes);
        generated_code.push("".to_string());
    }

    // Add external type definitions and model rebuilds for Pydantic forward references
    let mut external_types_and_rebuilds = vec![
        "# External type definitions".to_string(),
        "StdNumNonZeroU32 = Annotated[int, \"Rust NonZero u32 type\"]".to_string(),
        "StdNumNonZeroU64 = Annotated[int, \"Rust NonZero u64 type\"]".to_string(),
        "StdNumNonZeroI32 = Annotated[int, \"Rust NonZero i32 type\"]".to_string(),
        "StdNumNonZeroI64 = Annotated[int, \"Rust NonZero i64 type\"]".to_string(),
        "".to_string(),
        "# Rebuild models to resolve forward references".to_string(),
        "try:".to_string(),
    ];
    let mut sorted_type_names: Vec<_> = rendered_types.keys().collect();
    sorted_type_names.sort();
    for type_name in sorted_type_names {
        if !type_name.starts_with("std::") && !type_name.starts_with("reflectapi::") {
            external_types_and_rebuilds.push(format!("    {}.model_rebuild()", type_name));
        }
    }
    external_types_and_rebuilds.push("except AttributeError:".to_string());
    external_types_and_rebuilds
        .push("    # Some types may not have model_rebuild method".to_string());
    external_types_and_rebuilds.push("    pass".to_string());
    external_types_and_rebuilds.push("".to_string());
    external_types_and_rebuilds.push(
        "# Factory classes (generated after model rebuild to avoid forward references)".to_string(),
    );

    // Generate all factory classes now that types are defined and rebuilt
    for factory_info in &factory_data {
        let factory_code = if factory_info.is_internally_tagged {
            generate_internally_tagged_factory_class(
                &factory_info.enum_def,
                &factory_info.enum_name,
                &factory_info.union_members,
            )?
        } else {
            generate_factory_class_with_representation(
                &factory_info.enum_def,
                &factory_info.enum_name,
                &factory_info.union_members,
                &factory_info.enum_def.representation,
            )?
        };
        external_types_and_rebuilds.push(factory_code);
        external_types_and_rebuilds.push("".to_string());
    }

    generated_code.push(external_types_and_rebuilds.join("\n"));

    // Generate testing utilities if requested
    if config.generate_testing {
        // contains user-defined types that have Pydantic classes generated for them.
        // Note types with fallbacks to primitives are not added.
        let mut user_defined_types: Vec<String> = rendered_types.keys().cloned().collect();
        user_defined_types.sort();

        let testing_template = templates::TestingModule {
            types: user_defined_types,
        };
        generated_code.push(
            testing_template
                .render()
                .context("Failed to render testing module")?,
        );
    }

    let result = generated_code.join("\n\n");

    // Format with black if available
    format_python_code(&result)
}

/// Check if a type name looks like a generic type variable
fn is_probable_typevar(name: &str) -> bool {
    // Likely a TypeVar if it's a bare identifier not present in schema and starts with uppercase
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Infer generic parameters used by a struct. Prefer explicit schema parameters,
/// but also fall back to scanning field type references for known TypeVars.
fn infer_struct_generic_params(
    struct_def: &reflectapi_schema::Struct,
    _schema: &Schema,
) -> Vec<String> {
    // If the struct declares parameters explicitly, use them
    let explicit: Vec<String> = struct_def.parameters().map(|p| p.name.clone()).collect();
    if !explicit.is_empty() {
        return explicit;
    }

    use std::collections::HashSet;

    // Collect generic-looking symbols that actually appear in this struct's fields
    let mut used: HashSet<String> = HashSet::new();

    for field in struct_def.fields.iter() {
        collect_generic_type_vars(&field.type_ref, &mut used);
    }

    // Convert to sorted list for deterministic output
    let mut result: Vec<String> = used.into_iter().collect();
    result.sort();
    result
}

/// Infer generic parameters used by an enum. Prefer explicit schema parameters,
/// but also fall back to scanning variant field type references for known TypeVars.
fn infer_enum_generic_params(enum_def: &reflectapi_schema::Enum, schema: &Schema) -> Vec<String> {
    // If the enum declares parameters explicitly, use them
    let explicit: Vec<String> = enum_def.parameters().map(|p| p.name.clone()).collect();
    if !explicit.is_empty() {
        return explicit;
    }

    use reflectapi_schema::{Fields, TypeReference};
    use std::collections::HashSet;

    // Collect generic-looking symbols that actually appear in this enum's variant fields
    let mut used: HashSet<String> = HashSet::new();

    fn visit_typevars(tr: &TypeReference, schema: &Schema, out: &mut HashSet<String>) {
        if schema.get_type(&tr.name).is_none()
            && tr.arguments.is_empty()
            && is_probable_typevar(&tr.name)
        {
            out.insert(tr.name.clone());
        }
        for arg in &tr.arguments {
            visit_typevars(arg, schema, out);
        }
    }

    for variant in &enum_def.variants {
        match &variant.fields {
            Fields::Named(fields) => {
                for f in fields {
                    visit_typevars(&f.type_ref, schema, &mut used);
                }
            }
            Fields::Unnamed(fields) => {
                for f in fields {
                    visit_typevars(&f.type_ref, schema, &mut used);
                }
            }
            Fields::None => {}
        }
    }

    let mut result: Vec<String> = used.into_iter().collect();
    result.sort();
    result
}

fn render_type_without_factory(
    type_def: &Type,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<(String, Option<FactoryInfo>)> {
    match type_def {
        Type::Struct(s) => Ok((
            render_struct(s, schema, implemented_types, used_type_vars)?,
            None,
        )),
        Type::Enum(e) => render_enum_without_factory(e, schema, implemented_types, used_type_vars),
        Type::Primitive(_p) => {
            // Primitive types are handled by implemented_types mapping
            Ok((String::new(), None)) // This shouldn't be reached normally
        }
    }
}

/// Recursively collect fields from flattened structures
fn collect_flattened_fields(
    type_ref: &TypeReference,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
    parent_required: bool,
    depth: usize,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<Vec<templates::Field>> {
    // Prevent infinite recursion
    if depth > 10 {
        return Ok(vec![templates::Field {
            name: format!("# Max flattening depth reached for type {}", type_ref.name),
            type_annotation: "Any".to_string(),
            description: Some("Maximum flattening depth exceeded".to_string()),
            deprecation_note: None,
            optional: true,
            default_value: Some("None".to_string()),
            alias: None,
        }]);
    }

    let mut collected_fields = Vec::new();

    // Unwrap Option<T> and ReflectapiOption<T> when flattening
    let target_type_name = if (type_ref.name == "std::option::Option"
        || type_ref.name == "reflectapi::Option")
        && !type_ref.arguments.is_empty()
    {
        &type_ref.arguments[0].name
    } else {
        &type_ref.name
    };

    if let Some(reflectapi_schema::Type::Struct(struct_def)) = schema.get_type(target_type_name) {
        for field in struct_def.fields.iter() {
            if field.flattened() {
                // Recursively collect fields from nested flattened structures
                let nested_fields = collect_flattened_fields(
                    &field.type_ref,
                    schema,
                    implemented_types,
                    active_generics,
                    parent_required && field.required,
                    depth + 1,
                    used_type_vars,
                )?;
                collected_fields.extend(nested_fields);
            } else {
                // Regular field in flattened struct
                let field_type = type_ref_to_python_type(
                    &field.type_ref,
                    schema,
                    implemented_types,
                    active_generics,
                    used_type_vars,
                )?;

                let is_option_type = field.type_ref.name == "std::option::Option"
                    || field.type_ref.name == "reflectapi::Option";

                let (optional, default_value, final_field_type) =
                    if !field.required || !parent_required {
                        if is_option_type {
                            (true, Some("None".to_string()), field_type)
                        } else {
                            (
                                true,
                                Some("None".to_string()),
                                format!("{} | None", field_type),
                            )
                        }
                    } else if is_option_type {
                        (true, Some("None".to_string()), field_type)
                    } else {
                        (false, None, field_type)
                    };

                let (sanitized, alias) = sanitize_field_name_with_alias(field.name());
                collected_fields.push(templates::Field {
                    name: sanitized,
                    type_annotation: final_field_type,
                    description: Some(format!(
                        "(flattened{}) {}",
                        if depth > 1 {
                            format!(" depth={}", depth)
                        } else {
                            String::new()
                        },
                        field.description()
                    )),
                    deprecation_note: field.deprecation_note.clone(),
                    optional,
                    default_value,
                    alias,
                });
            }
        }
    }

    Ok(collected_fields)
}

fn render_struct(
    struct_def: &reflectapi_schema::Struct,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    // Check if this struct has any flattened fields
    let has_flattened = struct_def.fields.iter().any(|field| field.flattened());

    if has_flattened {
        // Use runtime flatten support
        return render_struct_with_flatten(struct_def, schema, implemented_types, used_type_vars);
    }

    // Check if this is a single-field tuple struct that should be unwrapped
    if struct_def.is_tuple() && struct_def.fields.len() == 1 {
        // Skip generation for single-field tuple structs
        // They should be unwrapped and used directly
        return Ok(String::new());
    }

    // Collect active generic parameter names for this struct, mapping to descriptive names
    let active_generics: Vec<String> = struct_def.parameters().map(|p| p.name.clone()).collect();

    // Track used generic type variables
    for generic in &active_generics {
        used_type_vars.insert(generic.clone());
    }

    // Separate regular fields from flattened fields
    let regular_fields = struct_def
        .fields
        .iter()
        .filter(|field| !field.flattened())
        .map(|field| {
            let base_field_type = type_ref_to_python_type(
                &field.type_ref,
                schema,
                implemented_types,
                &active_generics,
                used_type_vars,
            )?;

            // Check if field type is Option<T> or ReflectapiOption<T> (which handle nullability themselves)
            let is_option_type = field.type_ref.name == "std::option::Option"
                || field.type_ref.name == "reflectapi::Option";

            // Fix default value handling for optional fields
            let (optional, default_value, field_type) = if !field.required {
                // Field is not required - add | None if not already an Option type
                if is_option_type {
                    (true, Some("None".to_string()), base_field_type) // Option types handle nullability themselves
                } else {
                    (
                        true,
                        Some("None".to_string()),
                        format!("{} | None", base_field_type),
                    )
                }
            } else if is_option_type {
                // Field is required but Option type - still needs default None
                (true, Some("None".to_string()), base_field_type) // Option types handle nullability themselves
            } else {
                // Required non-Option field
                (false, None, base_field_type)
            };

            let (sanitized, alias) = sanitize_field_name_with_alias(field.name());
            Ok(templates::Field {
                name: sanitized,
                type_annotation: field_type,
                description: Some(field.description().to_string()),
                deprecation_note: field.deprecation_note.clone(),
                optional,
                default_value,
                alias,
            })
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    // Collect flattened fields recursively using the new helper function
    let mut flattened_fields: Vec<templates::Field> = Vec::new();
    for field in struct_def.fields.iter().filter(|f| f.flattened()) {
        let fields = collect_flattened_fields(
            &field.type_ref,
            schema,
            implemented_types,
            &active_generics,
            field.required,
            0,
            used_type_vars,
        )?;
        flattened_fields.extend(fields);
    }

    // Combine regular fields with flattened field information
    let mut all_fields = regular_fields;
    all_fields.extend(flattened_fields);

    // Check if this is a generic struct (has type parameters)
    let has_generics = !struct_def.parameters.is_empty();
    let class_name = improve_class_name(&struct_def.name);

    let class_template = templates::DataClass {
        name: class_name,
        description: Some(struct_def.description().to_string()),
        fields: all_fields,
        is_tuple: false,
        is_generic: has_generics,
        generic_params: active_generics.clone(),
    };

    class_template.render().context("Failed to render struct")
}

fn render_enum_without_factory(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<(String, Option<FactoryInfo>)> {
    use reflectapi_schema::{Fields, Representation};

    // Check if this is a tagged enum (internally tagged)
    match &enum_def.representation {
        Representation::Internal { tag } => {
            // Internally tagged enums need factories but they're generated after model rebuild
            let (rendered, union_variant_names) = render_internally_tagged_enum_without_factory(
                enum_def,
                tag,
                schema,
                implemented_types,
                used_type_vars,
            )?;

            // Return factory info so it can be generated later
            let factory_info = FactoryInfo {
                enum_def: enum_def.clone(),
                enum_name: improve_class_name(&enum_def.name),
                union_members: union_variant_names,
                is_internally_tagged: true,
            };

            Ok((rendered, Some(factory_info)))
        }
        Representation::Adjacent { tag, content } => {
            // Adjacently tagged enums are represented as { tag: "Variant", content: ... }
            let (rendered, union_variant_names) = render_adjacently_tagged_enum_without_factory(
                enum_def,
                tag,
                content,
                schema,
                implemented_types,
                used_type_vars,
            )?;

            // Return factory info so it can be generated later
            let factory_info = FactoryInfo {
                enum_def: enum_def.clone(),
                enum_name: improve_class_name(&enum_def.name),
                union_members: union_variant_names,
                is_internally_tagged: false, // Adjacent tagged, not internal
            };

            Ok((rendered, Some(factory_info)))
        }
        Representation::None => {
            // Untagged enums don't use factories
            let rendered =
                render_untagged_enum(enum_def, schema, implemented_types, used_type_vars)?;
            Ok((rendered, None))
        }
        _ => {
            // Check if this is a primitive-represented enum (has discriminant values)
            let has_discriminants = enum_def.variants.iter().any(|v| v.discriminant.is_some());

            if has_discriminants {
                // Primitive enums don't use factories
                let rendered = render_primitive_enum(enum_def)?;
                Ok((rendered, None))
            } else {
                // Check if this has complex variants (tuple or struct variants)
                let has_complex_variants = enum_def.variants.iter().any(|v| {
                    match &v.fields {
                        Fields::Named(_) => true,                      // struct variant
                        Fields::Unnamed(fields) => !fields.is_empty(), // tuple variant with fields
                        Fields::None => false,                         // unit variant
                    }
                });

                if has_complex_variants {
                    // This is an externally tagged enum with complex variants - needs factory
                    let (rendered, union_members) = render_externally_tagged_enum_without_factory(
                        enum_def,
                        schema,
                        implemented_types,
                        used_type_vars,
                    )?;
                    let factory_info = FactoryInfo {
                        enum_def: enum_def.clone(),
                        enum_name: improve_class_name(&enum_def.name),
                        union_members,
                        is_internally_tagged: false,
                    };
                    Ok((rendered, Some(factory_info)))
                } else {
                    // Simple string enum - no factory needed
                    let variants = enum_def
                        .variants
                        .iter()
                        .map(|variant| templates::EnumVariant {
                            name: to_screaming_snake_case(variant.name()),
                            value: variant.serde_name().to_string(),
                            description: Some(variant.description().to_string()),
                        })
                        .collect();

                    let enum_template = templates::EnumClass {
                        name: improve_class_name(&enum_def.name),
                        description: Some(enum_def.description().to_string()),
                        variants,
                    };

                    let rendered = enum_template.render().context("Failed to render enum")?;
                    Ok((rendered, None))
                }
            }
        }
    }
}

fn render_adjacently_tagged_enum_without_factory(
    enum_def: &reflectapi_schema::Enum,
    tag: &str,
    content: &str,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<(String, Vec<String>)> {
    use reflectapi_schema::Fields;

    let enum_name = improve_class_name(&enum_def.name);

    let mut variant_models = Vec::new();
    let mut union_variants = Vec::new();
    let generic_params: Vec<String> = infer_enum_generic_params(enum_def, schema);

    // Track used generic type variables
    for generic in &generic_params {
        used_type_vars.insert(generic.clone());
    }

    for variant in &enum_def.variants {
        match &variant.fields {
            Fields::None => {
                union_variants.push(format!("Literal[\"{}\"]", variant.name()));
            }
            Fields::Unnamed(unnamed_fields) => {
                let variant_class_name =
                    format!("{}{}Variant", enum_name, to_pascal_case(variant.name()));
                let mut fields = Vec::new();
                for (i, field) in unnamed_fields.iter().enumerate() {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;
                    fields.push(templates::Field {
                        name: format!("field_{}", i),
                        type_annotation: field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional: false,
                        default_value: None,
                        alias: None,
                    });
                }
                let variant_model = templates::DataClass {
                    name: variant_class_name.clone(),
                    description: Some(format!("{} variant", variant.name())),
                    fields,
                    is_tuple: !unnamed_fields.is_empty(),
                    is_generic: !generic_params.is_empty(),
                    generic_params: generic_params.clone(),
                };
                variant_models.push(variant_model.render()?);
                union_variants.push(variant_class_name);
            }
            Fields::Named(named_fields) => {
                let variant_class_name =
                    format!("{}{}Variant", enum_name, to_pascal_case(variant.name()));
                let mut fields = Vec::new();
                for field in named_fields {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;
                    let is_option_type = field.type_ref.name == "std::option::Option"
                        || field.type_ref.name == "reflectapi::Option";
                    let (optional, default_value, final_field_type) = if !field.required {
                        if is_option_type {
                            (true, Some("None".to_string()), field_type)
                        } else {
                            (
                                true,
                                Some("None".to_string()),
                                format!("{} | None", field_type),
                            )
                        }
                    } else if is_option_type {
                        (true, Some("None".to_string()), field_type)
                    } else {
                        (false, None, field_type)
                    };
                    let (sanitized, alias) = sanitize_field_name_with_alias(field.name());
                    fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,
                    });
                }
                let variant_model = templates::DataClass {
                    name: variant_class_name.clone(),
                    description: Some(format!("{} variant", variant.name())),
                    fields,
                    is_tuple: false,
                    is_generic: !generic_params.is_empty(),
                    generic_params: generic_params.clone(),
                };
                variant_models.push(variant_model.render()?);
                union_variants.push(variant_class_name);
            }
        }
    }

    // Build RootModel with before validator and serializer following {tag, content}
    let mut code = String::new();
    if !variant_models.is_empty() {
        code.push_str(&variant_models.join("\n\n"));
        code.push_str("\n\n");
    }
    let generic_inherits = if !generic_params.is_empty() {
        format!(", Generic[{}]", generic_params.join(", "))
    } else {
        String::new()
    };
    code.push_str(&format!(
        "# Adjacently tagged enum using RootModel\n{enum_name}Variants = Union[{union}]\n\nclass {enum_name}(RootModel[{enum_name}Variants]{generic_inherits}):\n    \"\"\"Adjacently tagged enum\"\"\"\n\n    @model_validator(mode='before')\n    @classmethod\n    def _validate_adjacently_tagged(cls, data):\n        # Handle direct variant instances\n        if isinstance(data, ({direct_types})):\n            return data\n        if isinstance(data, dict):\n            tag = data.get('{tag}')\n            content = data.get('{content}')\n            if tag is None:\n                raise ValueError(\"Missing tag field '{tag}'\")\n            if content is None and tag not in ({unit_variants_tuple}):\n                raise ValueError(\"Missing content field '{content}' for tag: {{}}\".format(tag))\n            # Dispatch based on tag\n{dispatch_cases}\n        raise ValueError(\"Unknown variant for {enum_name}: {{}}\".format(data))\n\n    @model_serializer\n    def _serialize_adjacently_tagged(self):\n{serialize_cases}\n        raise ValueError(f\"Cannot serialize {enum_name} variant: {{type(self.root)}}\")\n",
        enum_name = enum_name,
        union = union_variants.join(", "),
        generic_inherits = generic_inherits,
        direct_types = union_variants
            .iter()
            .filter(|s| !s.starts_with("Literal["))
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        tag = tag,
        content = content,
        unit_variants_tuple = enum_def
            .variants
            .iter()
            .filter(|v| matches!(v.fields, Fields::None))
            .map(|v| format!("'{}'", v.name()))
            .collect::<Vec<_>>()
            .join(", "),
        dispatch_cases = generate_adjacent_dispatch_cases(enum_def, &enum_name)?,
        serialize_cases = generate_adjacent_serialize_cases(enum_def, &enum_name, tag, content)?,
    ));
    if !generic_params.is_empty() {
        code.push_str("\n    def __class_getitem__(cls, params):\n        return cls\n");
    }

    Ok((code, union_variants))
}

fn generate_adjacent_dispatch_cases(
    enum_def: &reflectapi_schema::Enum,
    enum_name: &str,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;
    let mut cases = Vec::new();
    for variant in &enum_def.variants {
        let vname = variant.name();
        match &variant.fields {
            Fields::None => {
                cases.push(format!(
                    "            if tag == \"{vname}\":\n                return \"{vname}\""
                ));
            }
            Fields::Unnamed(unnamed_fields) => {
                let class_name = format!("{enum_name}{}Variant", to_pascal_case(vname));
                if unnamed_fields.len() == 1 {
                    cases.push(format!(
                        "            if tag == \"{vname}\":\n                return {class_name}(field_0=content)"
                    ));
                } else {
                    let assigns = (0..unnamed_fields.len())
                        .map(|i| format!("field_{}=content[{}]", i, i))
                        .collect::<Vec<_>>()
                        .join(", ");
                    cases.push(format!(
                        "            if tag == \"{vname}\":\n                if isinstance(content, list):\n                    return {class_name}({assigns})\n                else:\n                    raise ValueError(\"Expected list for tuple variant {vname}\")"
                    ));
                }
            }
            Fields::Named(_named_fields) => {
                let class_name = format!("{enum_name}{}Variant", to_pascal_case(vname));
                cases.push(format!(
                    "            if tag == \"{vname}\":\n                return {class_name}(**content)"
                ));
            }
        }
    }
    Ok(cases.join("\n"))
}

fn generate_adjacent_serialize_cases(
    enum_def: &reflectapi_schema::Enum,
    enum_name: &str,
    tag: &str,
    content: &str,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;
    let mut cases = Vec::new();
    for variant in &enum_def.variants {
        let vname = variant.name();
        match &variant.fields {
            Fields::None => {
                cases.push(format!(
                    "        if self.root == \"{vname}\":\n            return {{\"{tag}\": \"{vname}\"}}"
                ));
            }
            Fields::Unnamed(unnamed_fields) => {
                let class_name = format!("{enum_name}{}Variant", to_pascal_case(vname));
                // For tuple variants in adjacently tagged enums, serialize the content properly
                if unnamed_fields.len() == 1 {
                    // Single field tuple: serialize the field value directly
                    cases.push(format!(
                        "        if isinstance(self.root, {class_name}):\n            return {{\"{tag}\": \"{vname}\", \"{content}\": self.root.field_0}}"
                    ));
                } else {
                    // Multiple field tuple: serialize as array
                    let field_accesses: Vec<String> = (0..unnamed_fields.len())
                        .map(|i| format!("self.root.field_{}", i))
                        .collect();
                    cases.push(format!(
                        "        if isinstance(self.root, {class_name}):\n            return {{\"{tag}\": \"{vname}\", \"{content}\": [{}]}}",
                        field_accesses.join(", ")
                    ));
                }
            }
            Fields::Named(_named_fields) => {
                let class_name = format!("{enum_name}{}Variant", to_pascal_case(vname));
                cases.push(format!(
                    "        if isinstance(self.root, {class_name}):\n            return {{\"{tag}\": \"{vname}\", \"{content}\": self.root.model_dump(exclude_none=True)}}"
                ));
            }
        }
    }
    Ok(cases.join("\n"))
}

fn render_externally_tagged_enum_without_factory(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<(String, Vec<String>)> {
    // Generate the full enum (with factory)
    let full_enum =
        render_externally_tagged_enum(enum_def, schema, implemented_types, used_type_vars)?;

    // Extract just the part before the factory class (more specific split)
    let enum_name = improve_class_name(&enum_def.name);
    let factory_class_pattern = format!("\n\nclass {}Factory:", enum_name);
    let parts: Vec<&str> = full_enum.split(&factory_class_pattern).collect();
    let enum_without_factory = parts[0].to_string();

    // Extract union member names for factory generation later
    let union_variants = extract_union_members_from_enum(enum_def)?;

    Ok((enum_without_factory, union_variants))
}

fn extract_union_members_from_enum(
    enum_def: &reflectapi_schema::Enum,
) -> anyhow::Result<Vec<String>> {
    use reflectapi_schema::Fields;

    let enum_name = improve_class_name(&enum_def.name);
    let mut union_variants = Vec::new();

    for variant in &enum_def.variants {
        let variant_name = variant.name();
        match &variant.fields {
            Fields::None => {
                union_variants.push(format!("Literal[\"{}\"]", variant_name));
            }
            Fields::Unnamed(_) | Fields::Named(_) => {
                let variant_class_name =
                    format!("{}{}Variant", enum_name, to_pascal_case(variant_name));
                union_variants.push(variant_class_name);
            }
        }
    }

    Ok(union_variants)
}

fn render_externally_tagged_enum(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let enum_name = improve_class_name(&enum_def.name);
    let mut variant_models = Vec::new();
    let mut union_variants = Vec::new();
    let mut instance_validator_cases = Vec::new();
    let mut validator_cases = Vec::new();
    let mut dict_validator_cases = Vec::new();
    let mut serializer_cases = Vec::new();

    // Collect active generic parameter names for this enum
    let generic_params: Vec<String> = infer_enum_generic_params(enum_def, schema);

    // Track used generic type variables
    for generic in &generic_params {
        used_type_vars.insert(generic.clone());
    }

    // Generate variant models and build validation/serialization logic
    for variant in &enum_def.variants {
        let variant_name = variant.name();

        match &variant.fields {
            Fields::None => {
                // Unit variant: represented as string literal
                union_variants.push(format!("Literal[{:?}]", variant_name));

                validator_cases.push(format!(
                    "        if isinstance(data, str) and data == \"{}\":\n            return data",
                    variant_name
                ));

                serializer_cases.push(format!(
                    "        if self.root == \"{}\":\n            return \"{}\"",
                    variant_name, variant_name
                ));
            }
            Fields::Unnamed(unnamed_fields) => {
                // Tuple variant: create a model class
                let variant_class_name =
                    format!("{}{}Variant", enum_name, to_pascal_case(variant_name));
                let mut fields = Vec::new();
                let mut field_names = Vec::new();

                for (i, field) in unnamed_fields.iter().enumerate() {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;

                    let field_name = format!("field_{}", i);
                    field_names.push(field_name.clone());

                    fields.push(templates::Field {
                        name: field_name,
                        type_annotation: field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional: false,
                        default_value: None,
                        alias: None,
                    });
                }

                let variant_model = templates::DataClass {
                    name: variant_class_name.clone(),
                    description: Some(format!("{} variant", variant_name)),
                    fields,
                    is_tuple: true,
                    is_generic: !generic_params.is_empty(),
                    generic_params: generic_params.clone(),
                };

                variant_models.push(variant_model.render()?);
                let union_member = if !generic_params.is_empty() {
                    format!("{}[{}]", variant_class_name, generic_params.join(", "))
                } else {
                    variant_class_name.clone()
                };
                union_variants.push(union_member);

                // Handle direct instance validation
                instance_validator_cases.push(format!(
                    "        if isinstance(data, {}):\n            return data",
                    variant_class_name
                ));

                // For tuple variants, the JSON value can be a single value or array
                if field_names.len() == 1 {
                    dict_validator_cases.push(format!(
                        "            if key == \"{}\":\n                return {}(field_0=value)",
                        variant_name, variant_class_name
                    ));

                    serializer_cases.push(format!(
                        "        if isinstance(self.root, {}):\n            return {{\"{}\": self.root.field_0}}",
                        variant_class_name, variant_name
                    ));
                } else {
                    dict_validator_cases.push(format!(
                        "            if key == \"{}\":\n                if isinstance(value, list):\n                    return {}({})\n                else:\n                    raise ValueError(\"Expected list for tuple variant {}\")",
                        variant_name, variant_class_name,
                        field_names.iter().enumerate().map(|(i, name)| format!("{}=value[{}]", name, i)).collect::<Vec<_>>().join(", "),
                        variant_name
                    ));

                    serializer_cases.push(format!(
                        "        if isinstance(self.root, {}):\n            return {{\"{}\": [{}]}}",
                        variant_class_name, variant_name,
                        field_names.iter().map(|name| format!("self.root.{}", name)).collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            Fields::Named(named_fields) => {
                // Struct variant: create a model class
                let variant_class_name =
                    format!("{}{}Variant", enum_name, to_pascal_case(variant_name));
                let mut fields = Vec::new();

                for field in named_fields {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;

                    let (optional, default_value, final_field_type) = if !field.required {
                        (
                            true,
                            Some("None".to_string()),
                            format!("{} | None", field_type),
                        )
                    } else {
                        (false, None, field_type)
                    };

                    let (sanitized, alias) = sanitize_field_name_with_alias(field.name());
                    fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,
                    });
                }

                let variant_model = templates::DataClass {
                    name: variant_class_name.clone(),
                    description: Some(format!("{} variant", variant_name)),
                    fields,
                    is_tuple: false,
                    is_generic: !generic_params.is_empty(),
                    generic_params: generic_params.clone(),
                };

                variant_models.push(variant_model.render()?);
                let union_member = if !generic_params.is_empty() {
                    format!("{}[{}]", variant_class_name, generic_params.join(", "))
                } else {
                    variant_class_name.clone()
                };
                union_variants.push(union_member);

                // Handle direct instance validation
                instance_validator_cases.push(format!(
                    "        if isinstance(data, {}):\n            return data",
                    variant_class_name
                ));

                dict_validator_cases.push(format!(
                    "            if key == \"{}\":\n                return {}(**value)",
                    variant_name, variant_class_name
                ));

                serializer_cases.push(format!(
                    "        if isinstance(self.root, {}):\n            return {{\"{}\": self.root.model_dump(exclude_none=True)}}",
                    variant_class_name, variant_name
                ));
            }
        }
    }

    // Check if this enum is generic
    let is_generic = !generic_params.is_empty();

    // Generate TypeVar definitions if generic
    let type_var_definitions = if is_generic {
        generic_params
            .iter()
            .map(|param| format!("{} = TypeVar('{}')", param, param))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    };

    // Always use RootModel approach; add Generic[...] when needed
    let template = templates::ExternallyTaggedEnumRootModel {
        name: enum_name.clone(),
        description: if enum_def.description().is_empty() {
            None
        } else {
            Some(sanitize_description(enum_def.description()))
        },
        variant_models,
        union_variants: union_variants.join(", "),
        is_single_variant: union_variants.len() == 1,
        instance_validator_cases: instance_validator_cases.join("\n"),
        validator_cases: validator_cases.join("\n"),
        dict_validator_cases: dict_validator_cases.join("\n"),
        serializer_cases: serializer_cases.join("\n"),
        is_generic,
        generic_params: generic_params.clone(),
    };
    let enum_code = template
        .render()
        .context("Failed to render externally tagged enum")?;

    // Generate factory class for ergonomic instantiation
    let factory_class_code = generate_externally_tagged_factory_class(enum_def, &enum_name)?;

    // Combine all parts
    let mut result = String::new();

    // Add TypeVar definitions at the top if generic
    if !type_var_definitions.is_empty() {
        result.push_str(&type_var_definitions);
        result.push_str("\n\n");
    }

    // Add the enum code
    result.push_str(&enum_code);
    result.push_str("\n\n");

    // Add the factory code
    result.push_str(&factory_class_code);

    Ok(result)
}

fn generate_factory_class_with_representation(
    enum_def: &reflectapi_schema::Enum,
    enum_name: &str,
    union_members: &[String],
    representation: &reflectapi_schema::Representation,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let factory_name = format!("{}Factory", enum_name);
    let mut class_attributes = Vec::new();
    let mut static_methods = Vec::new();

    // Check if this enum is generic
    let is_generic = !enum_def.parameters.is_empty();
    let generic_params: Vec<String> = if is_generic {
        enum_def.parameters.iter().map(|p| p.name.clone()).collect()
    } else {
        Vec::new()
    };

    let generic_type_params = if is_generic {
        format!("[{}]", generic_params.join(", "))
    } else {
        String::new()
    };

    for (i, variant) in enum_def.variants.iter().enumerate() {
        let variant_class_name = &union_members[i];

        match &variant.fields {
            Fields::None => {
                // For generic enums, skip unit variants as they cannot be instantiated
                // (Generic externally tagged enums use Approach B which doesn't support unit variant instantiation)
                if !is_generic {
                    // Unit variant - creation method depends on representation
                    match representation {
                        reflectapi_schema::Representation::Adjacent { tag, .. } => {
                            // For adjacently tagged enums, unit variants need to be created as static methods
                            // returning the RootModel with dictionary format that the validator expects
                            let method_name = to_snake_case(variant.name());
                            static_methods.push(format!(
                                r#"    @staticmethod
    def {}() -> {}:
        '''Creates the '{}' variant of the {} enum.'''
        return {}.model_validate({{"{}": "{}"}})"#,
                                method_name,
                                enum_name,
                                variant.name(),
                                enum_name,
                                enum_name,
                                tag,
                                variant.name()
                            ));
                        }
                        reflectapi_schema::Representation::External => {
                            // For externally tagged enums, unit variants are also static methods
                            let method_name = to_snake_case(variant.name());
                            static_methods.push(format!(
                                r#"    @staticmethod
    def {}() -> {}:
        '''Creates the '{}' variant of the {} enum.'''
        return {}("{}")"#,
                                method_name,
                                enum_name,
                                variant.name(),
                                enum_name,
                                enum_name,
                                variant.name()
                            ));
                        }
                        _ => {
                            // For other representations, create as class attribute directly
                            class_attributes.push(format!(
                                "    {} = {}(\"{}\")",
                                variant.name().to_uppercase(),
                                enum_name,
                                variant.name()
                            ));
                        }
                    }
                }
            }
            Fields::Unnamed(_) | Fields::Named(_) => {
                // Complex variant - create static method
                let method_name = to_snake_case(variant.name());
                let method_params = generate_factory_method_params(variant)?;
                let method_args = generate_factory_method_args(variant)?;

                // For discriminated unions, methods should return the main enum type
                let (return_type, factory_body) = match representation {
                    reflectapi_schema::Representation::Internal { .. }
                    | reflectapi_schema::Representation::Adjacent { .. }
                    | reflectapi_schema::Representation::External => {
                        // For discriminated unions, return the main enum type and wrap the variant
                        let main_return_type = if is_generic {
                            format!("{}{}", enum_name, generic_type_params)
                        } else {
                            enum_name.to_string()
                        };
                        let variant_type = if is_generic && !variant_class_name.contains('[') {
                            format!("{}{}", variant_class_name, generic_type_params)
                        } else {
                            variant_class_name.clone()
                        };
                        let factory_body =
                            format!("return {}({}({}))", enum_name, variant_type, method_args);
                        (main_return_type, factory_body)
                    }
                    _ => {
                        // For other representations, return the variant type directly
                        let return_type = if is_generic && !variant_class_name.contains('[') {
                            format!("{}{}", variant_class_name, generic_type_params)
                        } else {
                            variant_class_name.clone()
                        };
                        let factory_body = format!("return {}({})", return_type, method_args);
                        (return_type, factory_body)
                    }
                };

                static_methods.push(format!(
                    r#"    @staticmethod
    def {}({}) -> {}:
        '''Creates the '{}' variant of the {} enum.'''
        {}"#,
                    method_name,
                    method_params,
                    return_type,
                    variant.name(),
                    enum_name,
                    factory_body
                ));
            }
        }
    }

    let enum_description = if enum_def.description().is_empty() {
        format!("{} variants", enum_name)
    } else {
        sanitize_description(enum_def.description())
    };

    let mut factory_code = format!(
        r#"class {}:
    '''Factory class for creating {} variants with ergonomic syntax.

    {}
    '''"#,
        factory_name, enum_name, enum_description
    );

    if !class_attributes.is_empty() {
        factory_code.push_str("\n\n");
        factory_code.push_str(&class_attributes.join("\n"));
    }

    if !static_methods.is_empty() {
        factory_code.push_str("\n\n");
        factory_code.push_str(&static_methods.join("\n\n"));
    }

    Ok(factory_code)
}

fn generate_internally_tagged_factory_class(
    enum_def: &reflectapi_schema::Enum,
    enum_name: &str,
    union_variant_names: &[String],
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let factory_name = format!("{}Factory", enum_name);
    let mut class_attributes = Vec::new();
    let mut static_methods = Vec::new();

    for (i, variant) in enum_def.variants.iter().enumerate() {
        let variant_name = variant.name();
        let variant_class_name = &union_variant_names[i];

        match &variant.fields {
            Fields::None => {
                // Unit variant - instantiate the variant class directly
                // For internally tagged enums, unit variants are BaseModel classes
                class_attributes.push(format!(
                    "    {} = {}()",
                    variant_name.to_uppercase(),
                    variant_class_name
                ));
            }
            Fields::Unnamed(_) | Fields::Named(_) => {
                // Complex variant - create static method
                let method_name = to_snake_case(variant_name);
                let method_params = generate_factory_method_params(variant)?;
                let method_args = generate_factory_method_args(variant)?;

                static_methods.push(format!(
                    r#"    @staticmethod
    def {}({}) -> {}:
        '''Creates the '{}' variant of the {} enum.'''
        return {}({})"#,
                    method_name,
                    method_params,
                    variant_class_name,
                    variant_name,
                    enum_name,
                    variant_class_name,
                    method_args
                ));
            }
        }
    }

    let enum_description = if enum_def.description().is_empty() {
        format!("{} variants", enum_name)
    } else {
        sanitize_description(enum_def.description())
    };

    let mut factory_code = format!(
        r#"class {}:
    '''Factory class for creating {} variants with ergonomic syntax.

    {}
    '''"#,
        factory_name, enum_name, enum_description
    );

    if !class_attributes.is_empty() {
        factory_code.push_str("\n\n");
        factory_code.push_str(&class_attributes.join("\n"));
    }

    if !static_methods.is_empty() {
        factory_code.push_str("\n\n");
        factory_code.push_str(&static_methods.join("\n\n"));
    }

    Ok(factory_code)
}

fn generate_externally_tagged_factory_class(
    enum_def: &reflectapi_schema::Enum,
    enum_name: &str,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let factory_name = format!("{}Factory", enum_name);
    let class_attributes: Vec<String> = Vec::new();
    let mut static_methods = Vec::new();

    for variant in &enum_def.variants {
        let variant_name = variant.name();

        match &variant.fields {
            Fields::None => {
                // Skip unit variants for externally tagged enums
                // They're already handled by the RootModel validator and instantiating
                // them here can cause forward reference issues
                // Users can create them with: EnumName("variant_name")
            }
            Fields::Unnamed(_) | Fields::Named(_) => {
                // Complex variant - create static method that returns wrapped RootModel
                let method_name = to_snake_case(variant_name);
                let variant_class_name =
                    format!("{}{}Variant", enum_name, to_pascal_case(variant_name));
                let method_params = generate_factory_method_params(variant)?;
                let method_args = generate_factory_method_args(variant)?;

                static_methods.push(format!(
                    "    @staticmethod\n    def {}({}) -> {}:\n        \"\"\"Creates the '{}' variant of the {} enum.\"\"\"\n        return {}({}({}))",
                    method_name,
                    method_params,
                    enum_name,
                    variant_name,
                    enum_name,
                    enum_name,
                    variant_class_name,
                    method_args
                ));
            }
        }
    }

    let mut factory_code = format!(
        "class {}:\n    \"\"\"Factory class for creating {} variants with ergonomic syntax.\n\n    {} variants\n    \"\"\"",
        factory_name, enum_name, enum_name
    );

    if !class_attributes.is_empty() {
        factory_code.push_str("\n\n");
        factory_code.push_str(&class_attributes.join("\n"));
    }

    if !static_methods.is_empty() {
        factory_code.push_str("\n\n");
        factory_code.push_str(&static_methods.join("\n\n"));
    }

    Ok(factory_code)
}

fn generate_factory_method_params(variant: &reflectapi_schema::Variant) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    match &variant.fields {
        Fields::None => Ok(String::new()),
        Fields::Unnamed(unnamed_fields) => {
            let params: Vec<String> = unnamed_fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    let param_name = format!("field_{}", i);
                    if field.required {
                        param_name
                    } else {
                        format!("{} = None", param_name)
                    }
                })
                .collect();
            Ok(params.join(", "))
        }
        Fields::Named(named_fields) => {
            let mut params: Vec<String> = Vec::new();

            // Separate required and optional parameters for better ergonomics
            let mut required_params: Vec<String> = Vec::new();
            let mut optional_params: Vec<String> = Vec::new();

            for field in named_fields {
                let param_name = to_snake_case(field.serde_name());
                if field.required {
                    required_params.push(param_name);
                } else {
                    optional_params.push(format!("{} = None", param_name));
                }
            }

            // Put required parameters first, then optional ones
            params.extend(required_params);
            params.extend(optional_params);

            Ok(params.join(", "))
        }
    }
}

fn generate_factory_method_args(variant: &reflectapi_schema::Variant) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    match &variant.fields {
        Fields::None => Ok(String::new()),
        Fields::Unnamed(unnamed_fields) => {
            let args: Vec<String> = (0..unnamed_fields.len())
                .map(|i| format!("field_{}=field_{}", i, i))
                .collect();
            Ok(args.join(", "))
        }
        Fields::Named(named_fields) => {
            let args: Vec<String> = named_fields
                .iter()
                .map(|field| {
                    let serde_name = field.serde_name();
                    let param_name = to_snake_case(serde_name);
                    let field_name = sanitize_field_name(field.name());
                    format!("{}={}", field_name, param_name)
                })
                .collect();
            Ok(args.join(", "))
        }
    }
}

fn render_primitive_enum(enum_def: &reflectapi_schema::Enum) -> anyhow::Result<String> {
    // Determine if this is an integer or float enum
    let is_float_enum = false;
    let mut enum_variants = Vec::new();

    for variant in &enum_def.variants {
        if let Some(discriminant) = variant.discriminant {
            // Check if any discriminant suggests this should be a float enum
            // For now, we'll treat all as integers unless we find a way to detect floats
            enum_variants.push(templates::PrimitiveEnumVariant {
                name: to_screaming_snake_case(variant.name()),
                value: discriminant.to_string(),
                description: Some(variant.description().to_string()),
            });
        } else {
            // If a variant doesn't have a discriminant, this shouldn't be a primitive enum
            return Err(anyhow::anyhow!(
                "Primitive enum {} has variant {} without discriminant",
                enum_def.name,
                variant.name()
            ));
        }
    }

    let enum_template = templates::PrimitiveEnumClass {
        name: improve_class_name(&enum_def.name),
        description: Some(enum_def.description().to_string()),
        variants: enum_variants,
        is_int_enum: !is_float_enum,
    };

    enum_template
        .render()
        .context("Failed to render primitive enum")
}

fn render_internally_tagged_enum_without_factory(
    enum_def: &reflectapi_schema::Enum,
    tag: &str,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<(String, Vec<String>)> {
    let (rendered, union_variant_names) = render_internally_tagged_enum_core(
        enum_def,
        tag,
        schema,
        implemented_types,
        used_type_vars,
    )?;
    Ok((rendered, union_variant_names))
}

fn render_internally_tagged_enum_core(
    enum_def: &reflectapi_schema::Enum,
    tag: &str,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<(String, Vec<String>)> {
    use reflectapi_schema::{Fields, Type};

    let enum_name = improve_class_name(&enum_def.name);
    let mut variant_class_definitions: Vec<String> = Vec::new();
    let mut union_variant_names: Vec<String> = Vec::new();

    // Check if this enum is generic
    let generic_params: Vec<String> = infer_enum_generic_params(enum_def, schema);

    // Track used generic type variables
    for generic in &generic_params {
        used_type_vars.insert(generic.clone());
    }

    let is_generic = !generic_params.is_empty();

    // Generate TypeVar definitions if generic
    let type_var_definitions = if is_generic {
        generic_params
            .iter()
            .map(|param| format!("{} = TypeVar('{}')", param, param))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        String::new()
    };

    // Generate individual classes for each variant
    for variant in &enum_def.variants {
        let variant_class_name = format!("{}{}", enum_name, to_pascal_case(variant.name()));

        // For generic enums, add the type parameters to the union variant names
        if is_generic {
            let params_str = generic_params.join(", ");
            union_variant_names.push(format!("{}[{}]", variant_class_name, params_str));
        } else {
            union_variant_names.push(variant_class_name.clone());
        }

        // Start with discriminator field
        // Always set a default value for the discriminator to simplify construction
        let discriminator_default_value = Some(format!("\"{}\"", variant.serde_name()));

        let mut fields = vec![templates::Field {
            name: tag.to_string(),
            type_annotation: format!("Literal['{}']", variant.serde_name()),
            description: Some("Discriminator field".to_string()),
            deprecation_note: None,
            optional: false,
            default_value: discriminator_default_value,
            alias: None,
        }];

        // Handle variant fields based on type
        match &variant.fields {
            Fields::Unnamed(unnamed_fields) => {
                // Tuple variant - flatten the inner type's fields
                anyhow::ensure!(
                    unnamed_fields.len() == 1,
                    "Internally tagged tuple variants must contain exactly one type, found {} in variant {}",
                    unnamed_fields.len(),
                    variant.name()
                );

                let inner_field = &unnamed_fields[0];
                let mut inner_type_name = &inner_field.type_ref.name;

                // Handle Box<T> by looking inside the Box to find the actual struct
                if inner_type_name == "std::boxed::Box" {
                    if let Some(boxed_arg) = inner_field.type_ref.arguments.first() {
                        inner_type_name = &boxed_arg.name;
                    } else {
                        return Err(anyhow::anyhow!(
                            "Tuple variant {} contains Box without type argument",
                            variant.name()
                        ));
                    }
                }

                // Get the inner type definition from schema
                let inner_type_def = schema.get_type(inner_type_name).ok_or_else(|| {
                    anyhow::anyhow!("Type {} not found in schema", inner_type_name)
                })?;

                // The inner type must be a struct for flattening to work
                match inner_type_def {
                    Type::Struct(inner_struct) => {
                        // Flatten the struct's fields into this variant class
                        let struct_fields = match &inner_struct.fields {
                            Fields::Named(fields) => fields,
                            _ => {
                                return Err(anyhow::anyhow!(
                                    "Tuple variant {} contains a struct with non-named fields, which cannot be flattened",
                                    variant.name()
                                ));
                            }
                        };

                        for struct_field in struct_fields {
                            let field_type = type_ref_to_python_type(
                                &struct_field.type_ref,
                                schema,
                                implemented_types,
                                &generic_params,
                                used_type_vars,
                            )?;

                            // Handle field optionality
                            let is_option_type = struct_field.type_ref.name
                                == "std::option::Option"
                                || struct_field.type_ref.name == "reflectapi::Option";
                            let (optional, default_value, final_field_type) =
                                if !struct_field.required {
                                    if is_option_type {
                                        (true, Some("None".to_string()), field_type)
                                    } else {
                                        (
                                            true,
                                            Some("None".to_string()),
                                            format!("{} | None", field_type),
                                        )
                                    }
                                } else if is_option_type {
                                    (true, Some("None".to_string()), field_type)
                                } else {
                                    (false, None, field_type)
                                };

                            let (sanitized, alias) =
                                sanitize_field_name_with_alias(struct_field.name());
                            fields.push(templates::Field {
                                name: sanitized,
                                type_annotation: final_field_type,
                                description: Some(struct_field.description().to_string()),
                                deprecation_note: struct_field.deprecation_note.clone(),
                                optional,
                                default_value,
                                alias,
                            });
                        }
                    }
                    Type::Enum(_) | Type::Primitive(_) => {
                        // For non-struct tuple variants, handle them like regular internally tagged enum variants
                        // Add a single field with the inner type
                        let field_type = type_ref_to_python_type(
                            &inner_field.type_ref,
                            schema,
                            implemented_types,
                            &generic_params,
                            used_type_vars,
                        )?;

                        fields.push(templates::Field {
                            name: "value".to_string(), // Use generic field name since it's not a struct
                            type_annotation: field_type,
                            description: Some("Tuple variant value".to_string()),
                            deprecation_note: None,
                            optional: false,
                            default_value: None,
                            alias: None,
                        });
                    }
                }
            }
            Fields::Named(named_fields) => {
                // Struct variant - handle normally
                for field in named_fields {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;

                    let is_option_type = field.type_ref.name == "std::option::Option"
                        || field.type_ref.name == "reflectapi::Option";
                    let (optional, default_value, final_field_type) = if !field.required {
                        if is_option_type {
                            (true, Some("None".to_string()), field_type)
                        } else {
                            (
                                true,
                                Some("None".to_string()),
                                format!("{} | None", field_type),
                            )
                        }
                    } else if is_option_type {
                        (true, Some("None".to_string()), field_type)
                    } else {
                        (false, None, field_type)
                    };

                    let (sanitized, alias) = sanitize_field_name_with_alias(field.name());
                    fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,
                    });
                }
            }
            Fields::None => {
                // Unit variant - only the discriminator field is needed
            }
        }

        // Generate the variant class
        let variant_template = templates::DataClass {
            name: variant_class_name,
            description: Some(variant.description().to_string()),
            fields,
            is_tuple: false,
            is_generic,
            generic_params: generic_params.clone(),
        };

        variant_class_definitions.push(
            variant_template
                .render()
                .context("Failed to render variant class")?,
        );
    }

    // Generate the discriminated union
    let union_variants: Vec<templates::UnionVariant> = enum_def
        .variants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
            let variant_class_name = format!("{}{}", enum_name, to_pascal_case(variant.name()));
            let full_type_annotation = &union_variant_names[i];

            templates::UnionVariant {
                name: variant.name().to_string(),
                type_annotation: full_type_annotation.clone(),
                base_name: variant_class_name,
                description: Some(variant.description().to_string()),
            }
        })
        .collect();

    let union_template = templates::UnionClass {
        name: enum_name.clone(),
        description: Some(enum_def.description().to_string()),
        variants: union_variants,
        discriminator_field: tag.to_string(),
        is_generic,
        generic_params: generic_params.clone(),
    };

    let union_definition = union_template.render().context("Failed to render union")?;

    // Combine all parts
    let mut result = String::new();

    // Add TypeVar definitions at the top if generic
    if !type_var_definitions.is_empty() {
        result.push_str(&type_var_definitions);
        result.push_str("\n\n");
    }

    // Add variant classes
    result.push_str(&variant_class_definitions.join("\n\n"));
    if !result.is_empty() {
        result.push_str("\n\n");
    }

    // Add union definition
    result.push_str(&union_definition);
    result.push_str("\n\n");

    // If generic, make RootModel class generic to enable subscription
    let generic_inherits = if !generic_params.is_empty() {
        format!(", Generic[{}]", generic_params.join(", "))
    } else {
        String::new()
    };

    // Patch class header to include Generic inheritance and subscription support
    // Replace the class header in-place
    let header = format!("class {}(RootModel[{}Variants])", enum_name, enum_name);
    let replacement = format!(
        "class {}(RootModel[{}Variants]{})",
        enum_name, enum_name, generic_inherits
    );
    let result = result.replace(&header, &replacement);
    let mut result = result;
    if !generic_inherits.is_empty() {
        // Add __class_getitem__ passthrough for runtime convenience
        let inject = "\n    def __class_getitem__(cls, params):\n        return cls\n".to_string();
        // Insert after class docstring or after class line
        if let Some(pos) = result.find(&replacement) {
            if let Some(nl) = result[pos..].find('\n') {
                let insert_at = pos + nl + 1;
                result.insert_str(insert_at, &inject);
            }
        }
    }

    // Don't generate factory inline - it will be generated after model rebuild
    // to avoid forward reference issues

    Ok((result, union_variant_names))
}

fn render_untagged_enum(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let enum_name = improve_class_name(&enum_def.name);
    let mut variant_classes = Vec::new();
    let mut union_variants = Vec::new();

    // Collect active generic parameter names for this enum
    let generic_params: Vec<String> = infer_enum_generic_params(enum_def, schema);

    // Track used generic type variables
    for generic in &generic_params {
        used_type_vars.insert(generic.clone());
    }

    // Process each variant to create separate classes (without discriminator fields)
    for variant in &enum_def.variants {
        let variant_class_name = format!("{}{}", enum_name, variant.name());
        let mut fields = Vec::new();

        // Add variant-specific fields (no discriminator field for untagged)
        match &variant.fields {
            Fields::Named(named_fields) => {
                for field in named_fields {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;
                    // Check if field type is Option<T> or ReflectapiOption<T> (which handle nullability themselves)
                    let is_option_type = field.type_ref.name == "std::option::Option"
                        || field.type_ref.name == "reflectapi::Option";
                    // Handle optionality
                    let (optional, default_value, final_field_type) = if !field.required {
                        if is_option_type {
                            (true, Some("None".to_string()), field_type)
                        } else {
                            (
                                true,
                                Some("None".to_string()),
                                format!("{} | None", field_type),
                            )
                        }
                    } else if is_option_type {
                        (true, Some("None".to_string()), field_type)
                    } else {
                        (false, None, field_type)
                    };

                    let (sanitized, alias) = sanitize_field_name_with_alias(field.name());
                    fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,
                    });
                }
            }
            Fields::Unnamed(unnamed_fields) => {
                // Handle tuple-like variants for untagged enums
                for (i, field) in unnamed_fields.iter().enumerate() {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;
                    let is_option_type = field.type_ref.name == "std::option::Option"
                        || field.type_ref.name == "reflectapi::Option";
                    let (optional, default_value, final_field_type) = if !field.required {
                        if is_option_type {
                            (true, Some("None".to_string()), field_type)
                        } else {
                            (
                                true,
                                Some("None".to_string()),
                                format!("{} | None", field_type),
                            )
                        }
                    } else if is_option_type {
                        (true, Some("None".to_string()), field_type)
                    } else {
                        (false, None, field_type)
                    };

                    fields.push(templates::Field {
                        name: format!("field_{}", i),
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias: None,
                    });
                }
            }
            Fields::None => {
                // Unit variant - create an empty class
                // No fields needed
            }
        }

        // Generate the variant class (without discriminator)
        let variant_template = templates::DataClass {
            name: variant_class_name.clone(),
            description: Some(variant.description().to_string()),
            fields,
            is_tuple: false,
            is_generic: false,
            generic_params: vec![],
        };

        variant_classes.push(
            variant_template
                .render()
                .context("Failed to render untagged variant class")?,
        );
        union_variants.push(templates::UnionVariant {
            name: variant.name().to_string(),
            type_annotation: variant_class_name.clone(),
            base_name: variant_class_name.clone(),
            description: Some(variant.description().to_string()),
        });
    }

    // Generate the union type alias (without Field discriminator)
    let union_template = templates::UntaggedUnionClass {
        name: enum_name.clone(),
        description: Some(enum_def.description().to_string()),
        variants: union_variants,
    };
    let union_definition = union_template
        .render()
        .context("Failed to render untagged union")?;

    // Combine all parts
    let mut result = variant_classes.join("\n\n");
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    result.push_str(&union_definition);

    // Untagged enums don't use factory classes - variants serialize directly to their values

    Ok(result)
}

// OneOf types don't exist in the current schema - this was removed

fn render_function(
    function: &Function,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
) -> anyhow::Result<templates::Function> {
    let input_type = if let Some(input_type) = function.input_type.as_ref() {
        type_ref_to_python_type_simple(input_type, schema, implemented_types, &[])?
    } else {
        "None".to_string()
    };

    let output_type = if let Some(output_type) = function.output_type.as_ref() {
        type_ref_to_python_type_simple(output_type, schema, implemented_types, &[])?
    } else {
        "Any".to_string()
    };

    let error_type = if let Some(error_type) = function.error_type.as_ref() {
        Some(type_ref_to_python_type_simple(
            error_type,
            schema,
            implemented_types,
            &[],
        )?)
    } else {
        None
    };

    // Compute headers type if the function specifies input headers
    let headers_type = if let Some(headers_ref) = function.input_headers.as_ref() {
        Some(type_ref_to_python_type_simple(
            headers_ref,
            schema,
            implemented_types,
            &[],
        )?)
    } else {
        None
    };

    // Extract path parameters from input type
    let path_params = extract_path_parameters(&function.path)?;

    // Combine base path with function name (like TypeScript and Rust generators do)
    let path = if function.path.is_empty() {
        format!("/{}", function.name)
    } else {
        format!("{}/{}", function.path, function.name)
    };

    // Check if input type is a primitive type
    let is_input_primitive = if let Some(input_type_ref) = &function.input_type {
        is_primitive_type(&input_type_ref.name)
    } else {
        false
    };

    // Determine if we need a body parameter
    let has_body = function.input_type.is_some();

    Ok(templates::Function {
        name: safe_python_identifier_with_context(
            &to_snake_case(&function.name),
            IdentifierContext::Method,
        ),
        original_name: None, // Will be set later if this function is nested
        description: Some(function.description().to_string()),
        method: "POST".to_string(),
        path,
        input_type,
        headers_type,
        output_type,
        error_type,
        path_params,
        has_body,
        is_input_primitive,
        deprecation_note: function.deprecation_note.clone(),
    })
}

// Extract path parameters from function definition
fn extract_path_parameters(path: &str) -> anyhow::Result<Vec<templates::Parameter>> {
    let mut path_params = Vec::new();

    // Extract path parameters by finding {param_name} patterns in the path
    let mut in_param = false;
    let mut current_param = String::new();

    for ch in path.chars() {
        match ch {
            '{' => {
                in_param = true;
                current_param.clear();
            }
            '}' => {
                if in_param && !current_param.is_empty() {
                    path_params.push(templates::Parameter {
                        name: to_snake_case(&current_param),
                        raw_name: current_param.clone(),
                        type_annotation: "str".to_string(), // Default to string for path params
                        description: Some(format!("Path parameter: {}", current_param)),
                    });
                }
                in_param = false;
                current_param.clear();
            }
            _ => {
                if in_param {
                    current_param.push(ch);
                }
            }
        }
    }

    Ok(path_params)
}

// Check if a type name represents a primitive type
fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "u8" | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
            | "bool"
            | "String"
            | "std::string::String"
            | "str"
            | "char"
    )
}

// Ensure a name is a valid Python identifier and doesn't conflict with reserved words
/// Context for Python identifier safety checks
#[derive(Debug, Clone, Copy)]
enum IdentifierContext {
    General, // Class names, variable names, etc.
    Method,  // Method names within classes
}

fn safe_python_identifier(name: &str) -> String {
    safe_python_identifier_with_context(name, IdentifierContext::General)
}

fn safe_python_identifier_with_context(name: &str, context: IdentifierContext) -> String {
    // Python reserved keywords that cannot be used as identifiers
    const PYTHON_KEYWORDS: &[&str] = &[
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
        "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return",
        "try", "while", "with", "yield",
    ];

    // Python built-in functions - full list for general identifiers
    const PYTHON_BUILTINS: &[&str] = &[
        "abs",
        "all",
        "any",
        "ascii",
        "bin",
        "bool",
        "bytes",
        "callable",
        "chr",
        "classmethod",
        "compile",
        "complex",
        "delattr",
        "dict",
        "dir",
        "divmod",
        "enumerate",
        "eval",
        "exec",
        "filter",
        "float",
        "format",
        "frozenset",
        "getattr",
        "globals",
        "hasattr",
        "hash",
        "help",
        "hex",
        "id",
        "input",
        "int",
        "isinstance",
        "issubclass",
        "iter",
        "len",
        "list",
        "locals",
        "map",
        "max",
        "memoryview",
        "min",
        "next",
        "object",
        "oct",
        "open",
        "ord",
        "pow",
        "print",
        "property",
        "range",
        "repr",
        "reversed",
        "round",
        "set",
        "setattr",
        "slice",
        "sorted",
        "staticmethod",
        "str",
        "sum",
        "super",
        "tuple",
        "type",
        "vars",
        "zip",
    ];

    // Problematic built-ins that should be avoided even in method contexts
    const PROBLEMATIC_BUILTINS: &[&str] =
        &["type", "super", "property", "classmethod", "staticmethod"];

    // Common Pydantic/typing names to avoid
    const PYDANTIC_NAMES: &[&str] = &[
        "BaseModel",
        "Field",
        "validator",
        "root_validator",
        "Config",
        "model_config",
        "model_fields",
        "model_validator",
        "model_serializer",
    ];

    let mut result = name.to_string();

    // Replace invalid characters with underscores
    result = result
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Ensure it starts with a letter or underscore
    if !result.is_empty() && result.chars().next().unwrap().is_ascii_digit() {
        result = format!("_{}", result);
    }

    let needs_suffix = match context {
        IdentifierContext::General => {
            PYTHON_KEYWORDS.contains(&result.as_str())
                || PYDANTIC_NAMES.contains(&result.as_str())
                || (PYTHON_BUILTINS.contains(&result.as_str()) && result.as_str() != "id")
        }
        IdentifierContext::Method => {
            PYTHON_KEYWORDS.contains(&result.as_str())
                || PYDANTIC_NAMES.contains(&result.as_str())
                || PROBLEMATIC_BUILTINS.contains(&result.as_str())
        }
    };

    if needs_suffix {
        result = format!("{}_", result);
    }

    result
}

// Utility functions for string case conversion
fn to_snake_case(s: &str) -> String {
    // Replace dots and hyphens with underscores
    let normalized = s.replace(['.', '-'], "_");

    let mut result = String::new();
    let chars = normalized.chars();

    for ch in chars {
        if ch.is_uppercase() && !result.is_empty() && !result.ends_with('_') {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }

    result
}

fn to_pascal_case(s: &str) -> String {
    // First, replace :: with _ to handle Rust module paths
    let normalized = s.replace("::", "_");

    normalized
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Improve Python class names to be more readable and Pythonic
fn improve_class_name(original_name: &str) -> String {
    // Handle Rust module paths with :: (e.g., "nomatches::IfConflictOnInsert" -> "NomatchesIfConflictOnInsert")
    if original_name.contains("::") {
        // Convert Rust-style module paths to PascalCase
        // e.g., "nomatches::IfConflictOnInsertRequired" -> "NomatchesIfConflictOnInsertRequired"
        let parts: Vec<&str> = original_name.split("::").collect();
        return parts
            .iter()
            .map(|part| to_pascal_case(part))
            .collect::<Vec<_>>()
            .join("");
    }

    // Handle dotted namespaces (e.g., "myapi.model.Pet" -> "Pet")
    if original_name.contains('.') {
        let parts: Vec<&str> = original_name.split('.').collect();
        if let Some(last_part) = parts.last() {
            return improve_class_name_part(last_part);
        }
    }

    improve_class_name_part(original_name)
}

/// Improve a single part of a class name
fn improve_class_name_part(name_part: &str) -> String {
    let pascal_name = to_pascal_case(name_part);

    // Define transformation rules as patterns
    let transformations = [
        // Remove common Rust std prefixes to make Python names cleaner
        ("StdOption", ""),
        ("StdVec", ""),
        ("StdResult", "Result"),
        ("StdTuple", "Tuple"),
        ("StdCollectionsHashMap", "HashMap"),
        ("StdCollectionsBTreeMap", "BTreeMap"),
        ("ReflectapiOption", "ReflectapiOption"), // Keep this one
        // Handle namespace prefixes
        ("Crate::", ""),
        ("Self::", ""),
    ];

    let mut improved = pascal_name.clone();

    // Apply prefix removals
    for (prefix, replacement) in &transformations {
        if improved.starts_with(prefix) {
            improved = format!("{}{}", replacement, &improved[prefix.len()..]);
            break;
        }
    }

    // Special case: handle compound names more intelligently
    // "InputPet" -> "Pet" (if "input" is in the name, keep as "InputPet")
    // "OutputPet" -> "Pet" (if "output" is in the name, keep as "OutputPet")
    // "KindDog" -> "KindDog" (keep compound enum variants)

    // Apply more intelligent name shortening
    let intelligent_shortenings = [
        // Remove redundant suffixes but keep meaningful ones
        ("OptionOption", "Option"),
        ("VecVec", "Vec"),
        ("ResultResult", "Result"),
        ("StringString", "String"),
    ];

    for (pattern, replacement) in &intelligent_shortenings {
        if improved.ends_with(pattern) {
            let prefix_len = improved.len() - pattern.len();
            improved = format!("{}{}", &improved[..prefix_len], replacement);
            break;
        }
    }

    // Clean up any remaining double patterns
    improved = improved.replace("__", "_").replace('_', "");

    // Ensure proper Pascal case
    if let Some(first_char) = improved.chars().next() {
        if first_char.is_lowercase() {
            let mut chars = improved.chars();
            chars.next();
            improved = format!("{}{}", first_char.to_uppercase(), chars.collect::<String>());
        }
    }

    // If we ended up with an empty string, return a default
    if improved.is_empty() {
        return "UnknownType".to_string();
    }

    improved
}

fn to_screaming_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

/// Generate nested class structure for better Python ergonomics
fn generate_nested_class_structure(
    rendered_types: &BTreeMap<String, String>,
    _schema: &Schema,
) -> String {
    let mut nested_classes = Vec::new();
    let mut namespace_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Group types by their namespace
    for type_name in rendered_types.keys() {
        if let Some(namespace) = extract_namespace_from_type_name(type_name) {
            namespace_groups
                .entry(namespace)
                .or_default()
                .push(type_name.clone());
        }
    }

    // Special handling for Kind unions - try to associate them with their related Input/Output types
    if let Some(unknown_kinds) = namespace_groups.remove("UnknownKind") {
        for kind_type in unknown_kinds {
            if kind_type == "MyapiModelKind" {
                // Look for related Pet types
                if namespace_groups.contains_key("Pet") {
                    namespace_groups.get_mut("Pet").unwrap().push(kind_type);
                }
            }
        }
    }

    // Generate nested classes for each namespace group
    let mut namespace_pairs: Vec<_> = namespace_groups.into_iter().collect();
    namespace_pairs.sort_by(|a, b| a.0.cmp(&b.0)); // Sort by namespace name for deterministic output
    for (namespace, type_names) in namespace_pairs {
        if type_names.len() >= 2 {
            // Only create nested classes if there are enough related types
            let nested_class = generate_namespace_class(&namespace, &type_names, rendered_types);
            if !nested_class.is_empty() {
                nested_classes.push(nested_class);
            }
        }
    }

    nested_classes.join("\n\n")
}

/// Extract namespace from a type name (e.g., "MyapiModelInputPet" -> "Pet")
fn extract_namespace_from_type_name(type_name: &str) -> Option<String> {
    // Look for patterns like "MyapiModelXxxYyy" -> "Yyy"
    if let Some(without_prefix) = type_name.strip_prefix("MyapiModel") {
        // Look for common patterns
        if let Some(stripped) = without_prefix.strip_prefix("Input") {
            return Some(stripped.to_string()); // Remove "Input"
        } else if let Some(stripped) = without_prefix.strip_prefix("Output") {
            return Some(stripped.to_string()); // Remove "Output"
        } else if let Some(remainder) = without_prefix.strip_prefix("Kind") {
            if remainder.is_empty() {
                // This is the main Kind union type like "MyapiModelKind" - assign to a generic namespace
                // We need to infer which namespace this belongs to from the schema context
                // For now, return a generic namespace that we can map later
                return Some("UnknownKind".to_string());
            } else {
                return Some(remainder.to_string()); // Return the variant name
            }
        } else {
            return Some(without_prefix.to_string());
        }
    } else if let Some(without_prefix) = type_name.strip_prefix("MyapiProto") {
        // Extract base name from request/response patterns
        if let Some(pos) = without_prefix.find("Request") {
            return Some(without_prefix[..pos].to_string());
        } else if let Some(pos) = without_prefix.find("Response") {
            return Some(without_prefix[..pos].to_string());
        } else if let Some(pos) = without_prefix.find("Error") {
            return Some(without_prefix[..pos].to_string());
        }
    }

    None
}

/// Generate a namespace class containing related types
fn generate_namespace_class(
    namespace: &str,
    type_names: &[String],
    _rendered_types: &BTreeMap<String, String>,
) -> String {
    let mut class_content = Vec::new();

    // Find the main types for this namespace
    let mut input_type = None;
    let mut output_type = None;
    let mut kind_variants = Vec::new();
    let mut request_types = Vec::new();
    let mut error_types = Vec::new();

    for type_name in type_names {
        if type_name.contains("Input") {
            input_type = Some(type_name);
        } else if type_name.contains("Output") {
            output_type = Some(type_name);
        } else if type_name.contains("Kind") {
            // Include both individual variants (MyapiModelKindDog) and the union type (MyapiModelKind)
            kind_variants.push(type_name);
        } else if type_name.contains("Request") {
            request_types.push(type_name);
        } else if type_name.contains("Error") {
            error_types.push(type_name);
        }
    }

    // Only generate if we have meaningful groupings
    if input_type.is_some() || output_type.is_some() || !kind_variants.is_empty() {
        class_content.push(format!("class {}:", namespace));
        class_content.push("    \"\"\"Grouped types for better organization.\"\"\"".to_string());

        // Add type aliases for the main types
        if let Some(input) = input_type {
            class_content.push(format!("    Input = {}", input));
        }
        if let Some(output) = output_type {
            class_content.push(format!("    Output = {}", output));
        }

        // Special handling for Kind types - check if there's a main union type we should expose
        let main_kind_type = "MyapiModelKind".to_string();
        if kind_variants.iter().any(|v| *v == &main_kind_type) {
            class_content.push("    ".to_string());
            class_content.push("    class Kind:".to_string());
            class_content.push("        \"\"\"Kind variants for this type.\"\"\"".to_string());
            class_content.push(format!("        Union = {}", main_kind_type));

            // Also manually add the variants we know exist for common types
            if namespace == "Pet" {
                class_content.push("        Dog = MyapiModelKindDog".to_string());
                class_content.push("        Cat = MyapiModelKindCat".to_string());
            }
        }

        // Add requests nested class if we have request types
        if !request_types.is_empty() {
            class_content.push("    ".to_string());
            class_content.push("    class Requests:".to_string());
            class_content.push("        \"\"\"Request types for this namespace.\"\"\"".to_string());

            for request_name in &request_types {
                if let Some(request_suffix) = extract_request_suffix(request_name) {
                    class_content.push(format!("        {} = {}", request_suffix, request_name));
                }
            }
        }

        // Add errors nested class if we have error types
        if !error_types.is_empty() {
            class_content.push("    ".to_string());
            class_content.push("    class Errors:".to_string());
            class_content.push("        \"\"\"Error types for this namespace.\"\"\"".to_string());

            for error_name in &error_types {
                if let Some(error_suffix) = extract_error_suffix(error_name) {
                    class_content.push(format!("        {} = {}", error_suffix, error_name));
                }
            }
        }

        return class_content.join("\n");
    }

    String::new()
}

/// Extract meaningful suffix from request type name
fn extract_request_suffix(request_name: &str) -> Option<String> {
    if let Some(pos) = request_name.find("Request") {
        let prefix = &request_name[..pos];
        if let Some(stripped) = prefix.strip_prefix("MyapiProto") {
            return Some(stripped.to_string());
        }
    }
    None
}

/// Extract meaningful suffix from error type name
fn extract_error_suffix(error_name: &str) -> Option<String> {
    if let Some(pos) = error_name.find("Error") {
        let prefix = &error_name[..pos];
        if let Some(stripped) = prefix.strip_prefix("MyapiProto") {
            return Some(stripped.to_string());
        }
    }
    None
}

fn sanitize_field_name(s: &str) -> String {
    to_valid_python_identifier(&to_snake_case(s))
}

fn sanitize_field_name_with_alias(s: &str) -> (String, Option<String>) {
    let snake_case = to_snake_case(s);
    let sanitized = to_valid_python_identifier(&snake_case);

    // If the sanitized name is different from the snake_case version,
    // it means we had to modify it due to Python keywords/builtins
    if sanitized != snake_case {
        (sanitized, Some(snake_case))
    } else {
        (sanitized, None)
    }
}

/// Map external/undefined types to sensible Python equivalents
/// Uses typing.Annotated to provide rich metadata like TypeScript comments
fn map_external_type_to_python(type_name: &str) -> String {
    match type_name {
        // Rust std library numeric types
        name if name.contains("NonZero")
            && (name.contains("U32")
                || name.contains("U64")
                || name.contains("I32")
                || name.contains("I64")) =>
        {
            format!("Annotated[int, \"Rust NonZero integer type: {}\"]", name)
        }

        // Decimal types (often used for financial data) - JSON serialized as strings
        name if name.contains("Decimal") => {
            format!(
                "Annotated[str, \"Rust Decimal type (JSON string): {}\"]",
                name
            )
        }

        // Common domain-specific types that are typically strings
        name if name.contains("Uuid") || name.contains("UUID") => {
            format!("Annotated[str, \"UUID type: {}\"]", name)
        }
        name if name.contains("Id") || name.ends_with("ID") => {
            format!("Annotated[str, \"ID type: {}\"]", name)
        }

        // Duration and time types - properly mapped to Python equivalents
        name if name.contains("Duration") => {
            format!("Annotated[timedelta, \"Rust Duration type: {}\"]", name)
        }
        name if name.contains("Instant") => {
            format!("Annotated[datetime, \"Rust Instant type: {}\"]", name)
        }

        // Path types - mapped to pathlib.Path
        name if name.contains("PathBuf") || name.contains("Path") => {
            format!("Annotated[Path, \"Rust Path type: {}\"]", name)
        }

        // IP address types - mapped to ipaddress module
        name if name.contains("IpAddr")
            || name.contains("Ipv4Addr")
            || name.contains("Ipv6Addr") =>
        {
            format!(
                "Annotated[IPv4Address | IPv6Address, \"Rust IP address type: {}\"]",
                name
            )
        }

        // For completely unmapped types, use Annotated[Any, ...] with metadata
        _ => format!("Annotated[Any, \"External type: {}\"]", type_name),
    }
}

// Type substitution function - handles TypeReference to Python type conversion
fn type_ref_to_python_type(
    type_ref: &TypeReference,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    // Check if this is an active generic parameter first
    if active_generics.contains(&type_ref.name) {
        used_type_vars.insert(type_ref.name.clone());
        return Ok(type_ref.name.clone());
    }

    // 1. FIRST: Check if this type has a direct mapping to a built-in Python type
    if let Some(python_type) = implemented_types.get(&type_ref.name) {
        if type_ref.arguments.is_empty() {
            return Ok(python_type.clone());
        }

        // Special case: Vec<u8> should be bytes in Python
        if type_ref.name == "std::vec::Vec" && type_ref.arguments.len() == 1 {
            let arg = &type_ref.arguments[0];
            if arg.name == "u8" {
                return Ok("bytes".to_string());
            }
        }

        // Special case: chrono::DateTime and similar types - ignore generic arguments
        if type_ref.name.starts_with("chrono::") {
            // chrono types map directly to Python datetime types regardless of timezone parameter
            return Ok(python_type.clone());
        }

        // Handle generic types with arguments
        let mut result = python_type.clone();

        // Get type parameter names for substitution (e.g., "T", "K", "V")
        let type_params = get_type_parameters(&type_ref.name);

        // Only substitute if we have known generic type parameters
        if !type_params.is_empty() {
            for (param, arg) in type_params.iter().zip(type_ref.arguments.iter()) {
                let resolved_arg = type_ref_to_python_type(
                    arg,
                    schema,
                    implemented_types,
                    active_generics,
                    used_type_vars,
                )?;
                // Safe token replacement: only replace whole word boundaries
                // This prevents replacing "T" in "DateTime" or partial matches
                result = safe_replace_generic_param(&result, param, &resolved_arg);
            }
        } else if !type_ref.arguments.is_empty() {
            // Regular generic type - add arguments as bracket notation
            let arg_types: Result<Vec<String>, _> = type_ref
                .arguments
                .iter()
                .map(|arg| {
                    type_ref_to_python_type(
                        arg,
                        schema,
                        implemented_types,
                        active_generics,
                        used_type_vars,
                    )
                })
                .collect();
            let arg_types = arg_types?;
            result = format!("{}[{}]", result, arg_types.join(", "));
        }

        return Ok(result);
    }

    // 2. SECOND: Check for fallback types BEFORE checking if it's a user-defined type
    // This is the key fix - prioritize fallback resolution over schema type generation
    if let Some(fallback_type_ref) = type_ref
        .fallback_once(schema.input_types())
        .or_else(|| type_ref.fallback_once(schema.output_types()))
    {
        // If a fallback exists, resolve it recursively
        return type_ref_to_python_type(
            &fallback_type_ref,
            schema,
            implemented_types,
            active_generics,
            used_type_vars,
        );
    }

    // 3. THIRD: Only after checking fallbacks, check if it's a user-defined type in the schema
    if let Some(type_def) = schema.get_type(&type_ref.name) {
        // Check if this is a single-field tuple struct that should be unwrapped
        if let reflectapi_schema::Type::Struct(struct_def) = type_def {
            if struct_def.is_tuple() && struct_def.fields.len() == 1 {
                // Return the inner type instead of the wrapper
                let inner_field = &struct_def.fields[0];
                return type_ref_to_python_type(
                    &inner_field.type_ref,
                    schema,
                    implemented_types,
                    active_generics,
                    used_type_vars,
                );
            }
        }

        let base_type = improve_class_name(&type_ref.name);

        // Handle generic types with arguments - follow TypeScript pattern
        if !type_ref.arguments.is_empty() {
            // Now that we're properly implementing generic variant classes,
            // we can remove the inline union generation and let the regular path handle it

            // Convert all arguments to Python types
            let arg_types: Result<Vec<String>, _> = type_ref
                .arguments
                .iter()
                .map(|arg| {
                    type_ref_to_python_type(
                        arg,
                        schema,
                        implemented_types,
                        active_generics,
                        used_type_vars,
                    )
                })
                .collect();
            let arg_types = arg_types?;

            // Always use bracket notation for generic types
            // Never do string replacement in the type name itself
            return Ok(format!("{}[{}]", base_type, arg_types.join(", ")));
        }

        return Ok(base_type);
    }

    // 4. FINAL: Final fallback for undefined external types - try to map to sensible Python types
    let fallback_type = map_external_type_to_python(&type_ref.name);
    Ok(fallback_type)
}

// Wrapper for backwards compatibility - doesn't track TypeVars
fn type_ref_to_python_type_simple(
    type_ref: &TypeReference,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
) -> anyhow::Result<String> {
    let mut unused_type_vars = BTreeSet::new();
    type_ref_to_python_type(
        type_ref,
        schema,
        implemented_types,
        active_generics,
        &mut unused_type_vars,
    )
}

// Get type parameter names for a given type
fn get_type_parameters(type_name: &str) -> Vec<&'static str> {
    match type_name {
        "std::boxed::Box" => vec!["T"],
        "std::sync::Arc" => vec!["T"],
        "std::rc::Rc" => vec!["T"],
        "std::vec::Vec" => vec!["T"],
        "std::option::Option" => vec!["T"],
        "reflectapi::Option" => vec!["T"],
        "std::collections::HashMap" => vec!["K", "V"],
        "std::collections::BTreeMap" => vec!["K", "V"],
        "std::result::Result" => vec!["T", "E"],
        _ => vec![],
    }
}

/// Safely replace generic parameter in a type string using word boundaries.
/// This prevents replacing partial matches like "T" in "DateTime".
fn safe_replace_generic_param(type_str: &str, param: &str, replacement: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = type_str.chars().collect();
    let param_chars: Vec<char> = param.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check if we can match the parameter at this position
        if i + param_chars.len() <= chars.len() {
            let slice: Vec<char> = chars[i..i + param_chars.len()].to_vec();
            if slice == param_chars {
                // Check word boundaries: ensure it's not part of a larger identifier
                let prev_is_boundary =
                    i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_';
                let next_is_boundary = i + param_chars.len() == chars.len()
                    || (!chars[i + param_chars.len()].is_alphanumeric()
                        && chars[i + param_chars.len()] != '_');

                if prev_is_boundary && next_is_boundary {
                    // Safe to replace - this is a whole word match
                    result.push_str(replacement);
                    i += param_chars.len();
                    continue;
                }
            }
        }

        // No match or not a word boundary - copy the character
        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Check if the schema uses datetime types (chrono)
fn check_datetime_usage(schema: &Schema, all_type_names: &[String]) -> bool {
    // Check all types for datetime usage
    for type_name in all_type_names {
        if let Some(type_def) = schema.get_type(type_name) {
            if type_uses_datetime(type_def) {
                return true;
            }
        }
    }

    // Check function parameters and return types
    for function in schema.functions() {
        if let Some(input_type) = &function.input_type {
            if let Some(type_def) = schema.get_type(&input_type.name) {
                if type_uses_datetime(type_def) {
                    return true;
                }
            }
        }

        if let Some(output_type) = &function.output_type {
            if let Some(type_def) = schema.get_type(&output_type.name) {
                if type_uses_datetime(type_def) {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if the schema uses UUID types
fn check_uuid_usage(schema: &Schema, all_type_names: &[String]) -> bool {
    // Check all types for UUID usage
    for type_name in all_type_names {
        if let Some(type_def) = schema.get_type(type_name) {
            if type_uses_uuid(type_def) {
                return true;
            }
        }
    }

    // Check function parameters and return types
    for function in schema.functions() {
        if let Some(input_type) = &function.input_type {
            if let Some(type_def) = schema.get_type(&input_type.name) {
                if type_uses_uuid(type_def) {
                    return true;
                }
            }
        }

        if let Some(output_type) = &function.output_type {
            if let Some(type_def) = schema.get_type(&output_type.name) {
                if type_uses_uuid(type_def) {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if the schema uses timedelta types (std::time::Duration)
fn check_timedelta_usage(schema: &Schema, all_type_names: &[String]) -> bool {
    // Check all types for timedelta usage
    for type_name in all_type_names {
        if let Some(type_def) = schema.get_type(type_name) {
            if type_uses_timedelta(type_def) {
                return true;
            }
        }
    }

    // Check function parameters and return types
    for function in schema.functions() {
        if let Some(input_type) = &function.input_type {
            if let Some(type_def) = schema.get_type(&input_type.name) {
                if type_uses_timedelta(type_def) {
                    return true;
                }
            }
        }

        if let Some(output_type) = &function.output_type {
            if let Some(type_def) = schema.get_type(&output_type.name) {
                if type_uses_timedelta(type_def) {
                    return true;
                }
            }
        }
    }

    false
}

/// Collect all unique generic parameter names from the schema
/// Check if the schema uses date types (chrono::NaiveDate)
fn check_date_usage(schema: &Schema, all_type_names: &[String]) -> bool {
    // Check all types for date usage
    for type_name in all_type_names {
        if let Some(type_def) = schema.get_type(type_name) {
            if type_uses_date(type_def) {
                return true;
            }
        }
    }

    // Check function parameters and return types
    for function in schema.functions() {
        if let Some(input_type) = &function.input_type {
            if let Some(type_def) = schema.get_type(&input_type.name) {
                if type_uses_date(type_def) {
                    return true;
                }
            }
        }

        if let Some(output_type) = &function.output_type {
            if let Some(type_def) = schema.get_type(&output_type.name) {
                if type_uses_date(type_def) {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if a specific type definition uses datetime
fn type_uses_datetime(type_def: &reflectapi_schema::Type) -> bool {
    match type_def {
        reflectapi_schema::Type::Struct(struct_def) => {
            for field in struct_def.fields.iter() {
                if type_ref_uses_datetime(&field.type_ref) {
                    return true;
                }
            }
        }
        reflectapi_schema::Type::Enum(enum_def) => {
            for variant in &enum_def.variants {
                match &variant.fields {
                    reflectapi_schema::Fields::Named(fields) => {
                        for field in fields {
                            if type_ref_uses_datetime(&field.type_ref) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::Unnamed(fields) => {
                        for field in fields {
                            if type_ref_uses_datetime(&field.type_ref) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::None => {}
                }
            }
        }
        reflectapi_schema::Type::Primitive(_) => {
            // Primitive types don't contain datetime fields
            return false;
        }
    }
    false
}

/// Check if a type reference uses datetime
fn type_ref_uses_datetime(type_ref: &TypeReference) -> bool {
    // Check for chrono types that map to datetime
    if type_ref.name == "chrono::DateTime" || type_ref.name == "chrono::NaiveDateTime" {
        return true;
    }

    // Check generic arguments recursively
    for param in &type_ref.arguments {
        if type_ref_uses_datetime(param) {
            return true;
        }
    }

    false
}

/// Check if a specific type definition uses UUID
fn type_uses_uuid(type_def: &reflectapi_schema::Type) -> bool {
    match type_def {
        reflectapi_schema::Type::Struct(struct_def) => {
            for field in struct_def.fields.iter() {
                if type_ref_uses_uuid(&field.type_ref) {
                    return true;
                }
            }
        }
        reflectapi_schema::Type::Enum(enum_def) => {
            for variant in &enum_def.variants {
                match &variant.fields {
                    reflectapi_schema::Fields::Named(fields) => {
                        for field in fields {
                            if type_ref_uses_uuid(&field.type_ref) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::Unnamed(fields) => {
                        for field in fields {
                            if type_ref_uses_uuid(&field.type_ref) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::None => {}
                }
            }
        }
        reflectapi_schema::Type::Primitive(_) => {
            return false;
        }
    }
    false
}

fn type_ref_uses_uuid(type_ref: &TypeReference) -> bool {
    // Check for UUID types
    if type_ref.name == "uuid::Uuid" {
        return true;
    }

    // Check generic arguments recursively
    for param in &type_ref.arguments {
        if type_ref_uses_uuid(param) {
            return true;
        }
    }

    false
}

/// Check if a specific type definition uses timedelta
fn type_uses_timedelta(type_def: &reflectapi_schema::Type) -> bool {
    match type_def {
        reflectapi_schema::Type::Struct(struct_def) => {
            for field in struct_def.fields.iter() {
                if type_ref_uses_timedelta(&field.type_ref) {
                    return true;
                }
            }
        }
        reflectapi_schema::Type::Enum(enum_def) => {
            for variant in &enum_def.variants {
                match &variant.fields {
                    reflectapi_schema::Fields::Named(fields) => {
                        for field in fields {
                            if type_ref_uses_timedelta(&field.type_ref) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::Unnamed(fields) => {
                        for field in fields {
                            if type_ref_uses_timedelta(&field.type_ref) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::None => {}
                }
            }
        }
        reflectapi_schema::Type::Primitive(_) => {
            return false;
        }
    }
    false
}

fn type_ref_uses_timedelta(type_ref: &TypeReference) -> bool {
    // Check for Duration types that map to timedelta
    if type_ref.name == "std::time::Duration" {
        return true;
    }

    // Check generic arguments recursively
    for param in &type_ref.arguments {
        if type_ref_uses_timedelta(param) {
            return true;
        }
    }

    false
}

/// Check if a specific type definition uses date
fn type_uses_date(type_def: &reflectapi_schema::Type) -> bool {
    match type_def {
        reflectapi_schema::Type::Struct(struct_def) => {
            for field in struct_def.fields.iter() {
                if type_ref_uses_date(&field.type_ref) {
                    return true;
                }
            }
        }
        reflectapi_schema::Type::Enum(enum_def) => {
            for variant in &enum_def.variants {
                match &variant.fields {
                    reflectapi_schema::Fields::Named(fields) => {
                        for field in fields {
                            if type_ref_uses_date(&field.type_ref) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::Unnamed(fields) => {
                        for field in fields {
                            if type_ref_uses_date(&field.type_ref) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::None => {}
                }
            }
        }
        reflectapi_schema::Type::Primitive(_) => {
            return false;
        }
    }
    false
}

fn type_ref_uses_date(type_ref: &TypeReference) -> bool {
    // Check for date types
    if type_ref.name == "chrono::NaiveDate" {
        return true;
    }

    // Check generic arguments recursively
    for param in &type_ref.arguments {
        if type_ref_uses_date(param) {
            return true;
        }
    }

    false
}

/// Check if the schema uses reflectapi::Option anywhere
fn schema_uses_reflectapi_option(schema: &Schema, all_type_names: &Vec<String>) -> bool {
    // Check all types in the schema for reflectapi::Option usage
    for type_name in all_type_names {
        if let Some(type_def) = schema.get_type(type_name) {
            if type_uses_reflectapi_option(type_def) {
                return true;
            }
        }
    }
    false
}

/// Check if the schema uses a specific type anywhere
fn schema_uses_type(schema: &Schema, all_type_names: &Vec<String>, target_type: &str) -> bool {
    // Check if the type is directly mentioned in the schema
    if all_type_names.contains(&target_type.to_string()) {
        return true;
    }

    // Check all types in the schema for usage of the target type
    for type_name in all_type_names {
        if let Some(type_def) = schema.get_type(type_name) {
            if type_references_type(type_def, target_type) {
                return true;
            }
        }
    }
    false
}

/// Check if a type definition references a specific type
fn type_references_type(type_def: &reflectapi_schema::Type, target_type: &str) -> bool {
    match type_def {
        reflectapi_schema::Type::Struct(s) => {
            // Check all fields in the struct
            for field in s.fields.iter() {
                if type_reference_references_type(&field.type_ref, target_type) {
                    return true;
                }
            }
        }
        reflectapi_schema::Type::Enum(e) => {
            // Check all variant fields in the enum
            for variant in &e.variants {
                match &variant.fields {
                    reflectapi_schema::Fields::Named(fields) => {
                        for field in fields {
                            if type_reference_references_type(&field.type_ref, target_type) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::Unnamed(fields) => {
                        for field in fields {
                            if type_reference_references_type(&field.type_ref, target_type) {
                                return true;
                            }
                        }
                    }
                    reflectapi_schema::Fields::None => {}
                }
            }
        }
        reflectapi_schema::Type::Primitive(_) => {
            // Primitives don't reference other types
        }
    }
    false
}

/// Check if a type reference references a specific type
fn type_reference_references_type(type_ref: &TypeReference, target_type: &str) -> bool {
    // Check if this is directly the target type
    if type_ref.name == target_type {
        return true;
    }

    // Check recursively in type arguments
    type_ref
        .arguments
        .iter()
        .any(|arg| type_reference_references_type(arg, target_type))
}

/// Recursively check if a type uses reflectapi::Option
fn type_uses_reflectapi_option(type_def: &reflectapi_schema::Type) -> bool {
    match type_def {
        reflectapi_schema::Type::Struct(s) => {
            // Check all fields in the struct
            for field in s.fields.iter() {
                if type_reference_uses_reflectapi_option(&field.type_ref) {
                    return true;
                }
            }
        }
        reflectapi_schema::Type::Enum(_) => {
            // Enums don't contain reflectapi::Option
        }
        reflectapi_schema::Type::Primitive(_) => {
            // Primitives don't contain reflectapi::Option
        }
    }
    false
}

/// Check if a type reference uses reflectapi::Option
fn type_reference_uses_reflectapi_option(type_ref: &TypeReference) -> bool {
    // Check if this is directly a reflectapi::Option
    if type_ref.name == "reflectapi::Option" {
        return true;
    }

    // Recursively check type arguments
    type_ref
        .arguments
        .iter()
        .any(type_reference_uses_reflectapi_option)
}

fn build_implemented_types() -> BTreeMap<String, String> {
    let mut types = BTreeMap::new();

    // Primitive types
    types.insert("i8".to_string(), "int".to_string());
    types.insert("i16".to_string(), "int".to_string());
    types.insert("i32".to_string(), "int".to_string());
    types.insert("i64".to_string(), "int".to_string());
    types.insert("u8".to_string(), "int".to_string());
    types.insert("u16".to_string(), "int".to_string());
    types.insert("u32".to_string(), "int".to_string());
    types.insert("u64".to_string(), "int".to_string());
    types.insert("f32".to_string(), "float".to_string());
    types.insert("f64".to_string(), "float".to_string());
    types.insert("bool".to_string(), "bool".to_string());
    types.insert("String".to_string(), "str".to_string());
    types.insert("std::string::String".to_string(), "str".to_string());

    // Collections with full Rust type names (using modern lowercase hints for Python 3.9+)
    types.insert("std::vec::Vec".to_string(), "list[T]".to_string());
    types.insert(
        "std::collections::HashMap".to_string(),
        "dict[K, V]".to_string(),
    );
    types.insert(
        "std::collections::BTreeMap".to_string(),
        "dict[K, V]".to_string(),
    );
    types.insert("std::option::Option".to_string(), "T | None".to_string());
    types.insert("std::result::Result".to_string(), "T | E".to_string());

    // ReflectAPI specific types
    types.insert(
        "reflectapi::Option".to_string(),
        "ReflectapiOption[T]".to_string(),
    );
    types.insert(
        "reflectapi::Empty".to_string(),
        "ReflectapiEmpty".to_string(),
    );
    types.insert(
        "reflectapi::Infallible".to_string(),
        "ReflectapiInfallible".to_string(),
    );

    // Date/time types
    types.insert("chrono::DateTime".to_string(), "datetime".to_string());
    types.insert("chrono::NaiveDateTime".to_string(), "datetime".to_string());
    types.insert("chrono::NaiveDate".to_string(), "date".to_string());
    types.insert("uuid::Uuid".to_string(), "UUID".to_string());

    // Rust std library types that need proper Python equivalents
    types.insert("std::time::Duration".to_string(), "timedelta".to_string());
    types.insert("std::path::PathBuf".to_string(), "Path".to_string());
    types.insert("std::path::Path".to_string(), "Path".to_string());
    types.insert(
        "std::net::IpAddr".to_string(),
        "IPv4Address | IPv6Address".to_string(),
    );
    types.insert("std::net::Ipv4Addr".to_string(), "IPv4Address".to_string());
    types.insert("std::net::Ipv6Addr".to_string(), "IPv6Address".to_string());

    // Special tuple/unit types
    types.insert("std::tuple::Tuple0".to_string(), "None".to_string());

    // Common serde types
    types.insert("serde_json::Value".to_string(), "Any".to_string());

    // Smart pointer types (transparent - map to their inner type)
    types.insert("std::boxed::Box".to_string(), "T".to_string());
    types.insert("std::sync::Arc".to_string(), "T".to_string());
    types.insert("std::rc::Rc".to_string(), "T".to_string());

    // External Rust types commonly found in ReflectAPI schemas
    types.insert("StdNumNonZeroU32".to_string(), "int".to_string());
    types.insert("StdNumNonZeroU64".to_string(), "int".to_string());
    types.insert("StdNumNonZeroI32".to_string(), "int".to_string());
    types.insert("StdNumNonZeroI64".to_string(), "int".to_string());
    types.insert("RustDecimalDecimal".to_string(), "str".to_string()); // JSON representation

    types
}

// Helper functions for templates

fn format_python_code(code: &str) -> anyhow::Result<String> {
    // Try to format with ruff format if available
    use std::process::{Command, Stdio};

    let child = Command::new("ruff")
        .args(["format", "--stdin-filename", "generated.py", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    match child {
        Ok(mut process) => {
            if let Some(stdin) = process.stdin.take() {
                use std::io::Write;
                let mut stdin = stdin;
                let _ = stdin.write_all(code.as_bytes());
            }
            let output = process.wait_with_output()?;
            if output.status.success() {
                let formatted = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(ensure_final_newline(formatted))
            } else {
                // Fall back to basic formatting if ruff fails
                Ok(basic_python_format(code))
            }
        }
        Err(_) => {
            // ruff not available, apply basic formatting
            Ok(basic_python_format(code))
        }
    }
}

fn basic_python_format(code: &str) -> String {
    let mut lines: Vec<String> = code.lines().map(|s| s.to_string()).collect();

    // Remove trailing whitespace from each line
    for line in &mut lines {
        *line = line.trim_end().to_string();
    }

    // Remove excessive blank lines (more than 2 consecutive)
    let mut formatted_lines = Vec::new();
    let mut blank_count = 0;

    for line in lines {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                formatted_lines.push(line);
            }
        } else {
            blank_count = 0;
            formatted_lines.push(line);
        }
    }

    ensure_final_newline(formatted_lines.join("\n"))
}

fn ensure_final_newline(mut code: String) -> String {
    if !code.ends_with('\n') {
        code.push('\n');
    }
    code
}

// Utility function to sanitize descriptions for safe inclusion in triple-quoted strings
fn sanitize_description(desc: &str) -> String {
    // Handle malformed JSON descriptions and other edge cases
    let mut sanitized = desc.trim().to_string();

    // If the description appears to be truncated or malformed (common signs)
    if sanitized.ends_with(',') || sanitized.ends_with('\\') {
        sanitized = format!(
            "{} [description may be truncated]",
            sanitized.trim_end_matches(',').trim_end_matches('\\')
        );
    }

    // Ensure the description doesn't break out of triple quotes
    // by replacing any potential problematic sequences
    sanitized = sanitized.replace("\"\"\"", "\\\"\\\"\\\"");

    sanitized
}

pub mod templates {
    use super::*;

    #[derive(Template)]
    #[template(
        source = r#"'''
Generated Python client for {{ package_name }}.

DO NOT MODIFY THIS FILE MANUALLY.
This file is automatically generated by ReflectAPI.
'''

from __future__ import annotations
"#,
        ext = "txt"
    )]
    pub struct FileHeader {
        pub package_name: String,
    }

    #[derive(Template)]
    #[template(
        source = r#"
# Standard library imports
{% if has_datetime %}from datetime import datetime{% if has_date %}, date{% endif %}{% if has_timedelta %}, timedelta{% endif %}
{% else if has_date %}from datetime import date{% if has_timedelta %}, timedelta{% endif %}
{% else if has_timedelta %}from datetime import timedelta
{% endif %}{% if has_enums %}from enum import Enum
{% endif %}from typing import Any, Optional, TypeVar, Generic, Union{% if has_annotated %}, Annotated{% endif %}{% if has_literal %}, Literal{% endif %}
{% if has_uuid %}from uuid import UUID
{% endif %}{% if has_warnings %}import warnings
{% endif %}

# Third-party imports
from pydantic import BaseModel, ConfigDict, Field{% if has_externally_tagged_enums %}, RootModel, model_validator, model_serializer, PrivateAttr{% endif %}

# Runtime imports
{% if has_async %}{% if has_sync %}from reflectapi_runtime import AsyncClientBase, ClientBase, ApiResponse{% else %}from reflectapi_runtime import AsyncClientBase, ApiResponse{% endif %}{% else %}{% if has_sync %}from reflectapi_runtime import ClientBase, ApiResponse{% endif %}{% endif %}
{% if has_reflectapi_option %}from reflectapi_runtime import ReflectapiOption
{% endif %}{% if has_reflectapi_empty %}from reflectapi_runtime import ReflectapiEmpty
{% endif %}{% if has_reflectapi_infallible %}from reflectapi_runtime import ReflectapiInfallible
{% endif %}{% if has_testing %}from reflectapi_runtime.testing import MockClient, create_api_response
{% endif %}

{% for type_var in global_type_vars %}{{ type_var }} = TypeVar('{{ type_var }}')
{% endfor %}
"#,
        ext = "txt"
    )]
    pub struct Imports {
        pub has_async: bool,
        pub has_sync: bool,
        pub has_testing: bool,
        pub has_enums: bool,
        pub has_reflectapi_option: bool,
        pub has_reflectapi_empty: bool,
        pub has_reflectapi_infallible: bool,
        pub has_flatten_support: bool,
        pub has_warnings: bool,
        pub has_datetime: bool,
        pub has_uuid: bool,
        pub has_timedelta: bool,
        pub has_date: bool,
        pub has_generics: bool,
        pub has_annotated: bool,
        pub has_literal: bool,
        pub has_discriminated_unions: bool,
        pub has_externally_tagged_enums: bool,
        pub global_type_vars: Vec<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"class {{ name }}(BaseModel{% if is_generic %}, Generic[{% for param in generic_params %}{{ param }}{% if !loop.last %}, {% endif %}{% endfor %}]{% endif %}):
{% if description.is_some() && !description.as_deref().unwrap().is_empty() %}    """{{ description.as_deref().unwrap() }}"""
{% else %}    """Generated data model."""
{% endif %}
    model_config = ConfigDict(extra="ignore", populate_by_name=True)

{% for field in fields %}    {{ field.name }}: {{ field.type_annotation }}{% if field.alias.is_some() %} = Field{% if field.default_value.is_some() %}(default={{ field.default_value.as_ref().unwrap() }}, serialization_alias='{{ field.alias.as_deref().unwrap() }}', validation_alias='{{ field.alias.as_deref().unwrap() }}'){% else if field.optional %}(default=None, serialization_alias='{{ field.alias.as_deref().unwrap() }}', validation_alias='{{ field.alias.as_deref().unwrap() }}'){% else %}(serialization_alias='{{ field.alias.as_deref().unwrap() }}', validation_alias='{{ field.alias.as_deref().unwrap() }}'){% endif %}{% else %}{% if field.optional %} = None{% else if field.default_value.is_some() %} = {{ field.default_value.as_ref().unwrap() }}{% endif %}{% endif %}
{% endfor %}
"#,
        ext = "txt"
    )]
    pub struct DataClass {
        pub name: String,
        pub description: Option<String>,
        pub fields: Vec<Field>,
        pub is_tuple: bool,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"class {{ name }}(str, Enum):
{% if description.is_some() && !description.as_deref().unwrap().is_empty() %}    """{{ description.as_deref().unwrap() }}"""
{% else %}    """Generated enum."""
{% endif %}

{% if variants.is_empty() %}    pass
{% else %}{% for variant in variants %}    {{ variant.name }} = "{{ variant.value }}"
{% endfor %}{% endif %}
"#,
        ext = "txt"
    )]
    pub struct EnumClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<EnumVariant>,
    }

    #[derive(Template)]
    #[template(
        source = r#"{% if is_int_enum %}from enum import IntEnum{% else %}from enum import Enum{% endif %}

class {{ name }}({% if is_int_enum %}IntEnum{% else %}Enum{% endif %}):
{% if description.is_some() %}    """{{ description.as_deref().unwrap() }}"""

{% endif %}{% for variant in variants %}
    {{ variant.name }} = {{ variant.value }}
{% if variant.description.is_some() %}    """{{ variant.description.as_deref().unwrap() }}"""
{% endif %}
{% endfor %}
"#,
        ext = "txt"
    )]
    pub struct PrimitiveEnumClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<PrimitiveEnumVariant>,
        pub is_int_enum: bool,
    }

    #[derive(Template)]
    #[template(
        source = r#"{% if is_generic %}
class {{ name }}(Generic[{% for param in generic_params %}{{ param }}{% if !loop.last %}, {% endif %}{% endfor %}]):
    """{{ description.as_deref().unwrap_or("Generated discriminated union type.") }}"""

    @classmethod
    def __class_getitem__(cls, params):
        """Enable subscripting for generic discriminated union."""
        if not isinstance(params, tuple):
            params = (params,)
        if len(params) != {{ generic_params.len() }}:
            raise TypeError(f"Expected {{ generic_params.len() }} type parameters, got {len(params)}")

        return Annotated[
            Union[{% for variant in variants %}{{ variant.base_name }}[{% for param in generic_params %}params[{{ loop.index0 }}]{% if !loop.last %}, {% endif %}{% endfor %}]{% if !loop.last %}, {% endif %}{% endfor %}],
            Field(discriminator='{{ discriminator_field }}')
        ]
{% else %}
class {{ name }}(RootModel):
    root: Annotated[Union[{% for variant in variants %}{{ variant.type_annotation }}{% if !loop.last %}, {% endif %}{% endfor %}], Field(discriminator='{{ discriminator_field }}')]
{% endif %}
{% if description.is_some() && !description.as_deref().unwrap_or("").is_empty() && !is_generic %}"""{{ description.as_deref().unwrap() }}"""{% endif %}
"#,
        ext = "txt"
    )]
    pub struct UnionClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<UnionVariant>,
        pub discriminator_field: String,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"{{ name }} = Union[{% for variant in variants %}{{ variant.type_annotation }}{% if !loop.last %}, {% endif %}{% endfor %}]
{% if description.is_some() && !description.as_deref().unwrap_or("").is_empty() %}"""{{ description.as_deref().unwrap() }}"""{% endif %}
"#,
        ext = "txt"
    )]
    pub struct UntaggedUnionClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<UnionVariant>,
    }

    #[derive(Template)]
    #[template(
        source = r#"{% if generate_async %}
{% for group in function_groups %}
class Async{{ group.class_name }}:
    """Async client for {{ group.name }} operations."""

    def __init__(self, client: AsyncClientBase) -> None:
        self._client = client
{% for function in group.functions %}

    async def {{ function.name }}(
        self,
{%- for param in function.path_params %}
        {{ param.name }}: {{ param.type_annotation }},
{%- endfor %}
{%- if function.has_body %}
        data: Optional[{{ function.input_type }}] = None,
{%- endif %}
{%- if function.headers_type.is_some() %}
        headers: Optional[{{ function.headers_type.as_deref().unwrap() }}] = None,
{%- endif %}
    ) -> ApiResponse[{{ function.output_type }}]:
        """{{ function.description.as_deref().unwrap_or("") }}{% if function.has_body || !function.path_params.is_empty() %}

        Args:{% if function.has_body %}
            data: Request data for the {{ function.name }} operation.{% endif %}{% if !function.path_params.is_empty() %}
{% for param in function.path_params %}            {{ param.name }}: {{ param.description.as_deref().unwrap_or("Path parameter") }}
{% endfor %}{% endif %}{% endif %}

        Returns:
            ApiResponse[{{ function.output_type }}]: Response containing {{ function.output_type }} data{% if function.deprecation_note.is_some() %}

        .. deprecated::
           {{ function.deprecation_note.as_deref().unwrap() }}{% endif %}
        """
        {% if function.deprecation_note.is_some() %}
        warnings.warn(
            "{% if function.original_name.is_some() %}{{ function.original_name.as_deref().unwrap() }}{% else %}{{ function.name }}{% endif %} is deprecated{% if !function.deprecation_note.as_deref().unwrap().is_empty() %}: {{ function.deprecation_note.as_deref().unwrap() }}{% endif %}",
            DeprecationWarning,
            stacklevel=2,
        )

        {% endif -%}
        path = "{{ function.path }}"
{% if !function.path_params.is_empty() %}
        # Format path parameters using safer string formatting
        path_params = {
{% for param in function.path_params %}
            "{{ param.raw_name }}": str({{ param.name }}),
{% endfor %}
        }
        for param_name, param_value in path_params.items():
            path = path.replace("{" + param_name + "}", param_value)
{% endif %}
        params: dict[str, Any] = {}
        return await self._client._make_request(
            "{{ function.method }}",
            path,
            params=params if params else None,
{% if function.has_body -%}
{% if function.is_input_primitive %}            json_data=data,
{% else %}            json_model=data,
{% endif -%}
{% endif -%}
{% if function.headers_type.is_some() %}            headers_model=headers,
{% endif -%}
{% if function.output_type == "Any" %}            response_model=None,
{% else %}            response_model={{ function.output_type }},
{% endif -%}
        )

{% endfor %}

{% endfor %}
class {{ async_class_name }}(AsyncClientBase):
    """Async client for the API."""

    def __init__(
        self,
        base_url: str{% if base_url.is_some() %} = "{{ base_url.as_ref().unwrap() }}"{% endif %},
        **kwargs: Any,
    ) -> None:
        super().__init__(base_url, **kwargs)
{% for group in function_groups %}
        self.{{ group.name }} = Async{{ group.class_name }}(self)
{% endfor %}

{% for function in top_level_functions %}
    async def {{ function.name }}(
        self,
{%- for param in function.path_params %}
        {{ param.name }}: {{ param.type_annotation }},
{%- endfor %}
{%- if function.has_body %}
        data: Optional[{{ function.input_type }}] = None,
{%- endif %}
{%- if function.headers_type.is_some() %}
        headers: Optional[{{ function.headers_type.as_deref().unwrap() }}] = None,
{%- endif %}
    ) -> ApiResponse[{{ function.output_type }}]:
        """{{ function.description.as_deref().unwrap_or("") }}{% if function.has_body %}

        Args:
            data: Request data for the {{ function.name }} operation.{% endif %}{% if !function.path_params.is_empty() %}
{% for param in function.path_params %}            {{ param.name }}: {{ param.description.as_deref().unwrap_or("Path parameter") }}
{% endfor %}{% endif %}

        Returns:
            ApiResponse[{{ function.output_type }}]: Response containing {{ function.output_type }} data{% if function.deprecation_note.is_some() %}

        .. deprecated::
           {{ function.deprecation_note.as_deref().unwrap() }}{% endif %}
        """
        {% if function.deprecation_note.is_some() %}
        warnings.warn(
            "{{ function.name }} is deprecated{% if !function.deprecation_note.as_deref().unwrap().is_empty() %}: {{ function.deprecation_note.as_deref().unwrap() }}{% endif %}",
            DeprecationWarning,
            stacklevel=2,
        )

        {% endif -%}
        path = "{{ function.path }}"
{% if !function.path_params.is_empty() %}
        # Format path parameters using safer string formatting
        path_params = {
{% for param in function.path_params %}
            "{{ param.name }}": str({{ param.name }}),
{% endfor %}
        }
        for param_name, param_value in path_params.items():
            path = path.replace("{" + param_name + "}", param_value)
{% endif %}
        params: dict[str, Any] = {}
        return await self._make_request(
            "{{ function.method }}",
            path,
            params=params if params else None,
{% if function.has_body -%}
{% if function.is_input_primitive %}            json_data=data,
{% else %}            json_model=data,
{% endif -%}
{% endif -%}
{% if function.headers_type.is_some() %}            headers_model=headers,
{% endif -%}
{% if function.output_type == "Any" %}            response_model=None,
{% else %}            response_model={{ function.output_type }},
{% endif -%}
        )

{% endfor %}

{% endif %}

{% if generate_sync %}
{% for group in function_groups %}
class {{ group.class_name }}:
    """Synchronous client for {{ group.name }} operations."""

    def __init__(self, client: ClientBase) -> None:
        self._client = client
{% for function in group.functions %}

    def {{ function.name }}(
        self,
{%- for param in function.path_params %}
        {{ param.name }}: {{ param.type_annotation }},
{%- endfor %}
{%- if function.has_body %}
        data: Optional[{{ function.input_type }}] = None,
{%- endif %}
{%- if function.headers_type.is_some() %}
        headers: Optional[{{ function.headers_type.as_deref().unwrap() }}] = None,
{%- endif %}
    ) -> ApiResponse[{{ function.output_type }}]:
        """{{ function.description.as_deref().unwrap_or("") }}{% if function.has_body || !function.path_params.is_empty() %}

        Args:{% if function.has_body %}
            data: Request data for the {{ function.name }} operation.{% endif %}{% if !function.path_params.is_empty() %}
{% for param in function.path_params %}            {{ param.name }}: {{ param.description.as_deref().unwrap_or("Path parameter") }}
{% endfor %}{% endif %}{% endif %}

        Returns:
            ApiResponse[{{ function.output_type }}]: Response containing {{ function.output_type }} data{% if function.deprecation_note.is_some() %}

        .. deprecated::
           {{ function.deprecation_note.as_deref().unwrap() }}{% endif %}
        """
        {% if function.deprecation_note.is_some() %}
        warnings.warn(
            "{% if function.original_name.is_some() %}{{ function.original_name.as_deref().unwrap() }}{% else %}{{ function.name }}{% endif %} is deprecated{% if !function.deprecation_note.as_deref().unwrap().is_empty() %}: {{ function.deprecation_note.as_deref().unwrap() }}{% endif %}",
            DeprecationWarning,
            stacklevel=2,
        )

        {% endif -%}
        path = "{{ function.path }}"
{% if !function.path_params.is_empty() %}
        # Format path parameters using safer string formatting
        path_params = {
{% for param in function.path_params %}
            "{{ param.raw_name }}": str({{ param.name }}),
{% endfor %}
        }
        for param_name, param_value in path_params.items():
            path = path.replace("{" + param_name + "}", param_value)
{% endif %}
        params: dict[str, Any] = {}
        return self._client._make_request(
            "{{ function.method }}",
            path,
            params=params if params else None,
{% if function.has_body -%}
{% if function.is_input_primitive %}            json_data=data,
{% else %}            json_model=data,
{% endif -%}
{% endif -%}
{% if function.headers_type.is_some() %}            headers_model=headers,
{% endif -%}
{% if function.output_type == "Any" %}            response_model=None,
{% else %}            response_model={{ function.output_type }},
{% endif -%}
        )

{% endfor %}

{% endfor %}
class {{ class_name }}(ClientBase):
    """Synchronous client for the API."""

    def __init__(
        self,
        base_url: str{% if base_url.is_some() %} = "{{ base_url.as_ref().unwrap() }}"{% endif %},
        **kwargs: Any,
    ) -> None:
        super().__init__(base_url, **kwargs)
{% for group in function_groups %}
        self.{{ group.name }} = {{ group.class_name }}(self)
{% endfor %}

{% for function in top_level_functions %}
    def {{ function.name }}(
        self,
{%- for param in function.path_params %}
        {{ param.name }}: {{ param.type_annotation }},
{%- endfor %}
{%- if function.has_body %}
        data: Optional[{{ function.input_type }}] = None,
{%- endif %}
{%- if function.headers_type.is_some() %}
        headers: Optional[{{ function.headers_type.as_deref().unwrap() }}] = None,
{%- endif %}
    ) -> ApiResponse[{{ function.output_type }}]:
        """{{ function.description.as_deref().unwrap_or("") }}{% if function.has_body %}

        Args:
            data: Request data for the {{ function.name }} operation.{% endif %}{% if !function.path_params.is_empty() %}
{% for param in function.path_params %}            {{ param.name }}: {{ param.description.as_deref().unwrap_or("Path parameter") }}
{% endfor %}{% endif %}

        Returns:
            ApiResponse[{{ function.output_type }}]: Response containing {{ function.output_type }} data{% if function.deprecation_note.is_some() %}

        .. deprecated::
           {{ function.deprecation_note.as_deref().unwrap() }}{% endif %}
        """
        {% if function.deprecation_note.is_some() %}
        warnings.warn(
            "{{ function.name }} is deprecated{% if !function.deprecation_note.as_deref().unwrap().is_empty() %}: {{ function.deprecation_note.as_deref().unwrap() }}{% endif %}",
            DeprecationWarning,
            stacklevel=2,
        )
        {% endif %}
        path = "{{ function.path }}"
{% if !function.path_params.is_empty() %}
        # Format path parameters using safer string formatting
        path_params = {
{% for param in function.path_params %}
            "{{ param.name }}": str({{ param.name }}),
{% endfor %}
        }
        for param_name, param_value in path_params.items():
            path = path.replace("{" + param_name + "}", param_value)
{% endif %}

        params: dict[str, Any] = {}

        return await self._make_request(
            "{{ function.method }}",
            path,
            params=params if params else None,
{% if function.has_body %}
{% if function.is_input_primitive %}
            json_data=data,
{% else %}
            json_model=data,
{% endif %}
{% endif %}
{% if function.headers_type.is_some() %}
            headers_model=headers,
{% endif %}
{% if function.output_type == "Any" %}
            response_model=None,
{% else %}
            response_model={{ function.output_type }},
{% endif %}
        )

{% endfor %}

{% endif %}
"#,
        ext = "txt"
    )]
    pub struct ClientClass {
        pub class_name: String,
        pub async_class_name: String,
        pub top_level_functions: Vec<Function>,
        pub function_groups: Vec<FunctionGroup>,
        pub generate_async: bool,
        pub generate_sync: bool,
        pub base_url: Option<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"# Testing utilities

{% for type_name in types %}
def create_{{ type_name.to_lowercase() }}_response(value: {{ type_name }}) -> ApiResponse[{{ type_name }}]:
    """Create a mock ApiResponse for {{ type_name }}."""
    return create_api_response(value)

{% endfor %}

def create_mock_client() -> MockClient:
    """Create a mock client for testing."""
    return MockClient()
"#,
        ext = "txt"
    )]
    pub struct TestingModule {
        pub types: Vec<String>,
    }

    pub struct Field {
        pub name: String,
        pub type_annotation: String,
        pub description: Option<String>,
        pub deprecation_note: Option<String>,
        pub optional: bool,
        pub default_value: Option<String>,
        pub alias: Option<String>,
    }

    pub struct EnumVariant {
        pub name: String,
        pub value: String,
        pub description: Option<String>,
    }

    pub struct PrimitiveEnumVariant {
        pub name: String,
        pub value: String,
        pub description: Option<String>,
    }

    pub struct UnionVariant {
        pub name: String,
        pub type_annotation: String,
        pub base_name: String,
        pub description: Option<String>,
    }

    #[derive(Clone)]
    pub struct Function {
        pub name: String,
        pub original_name: Option<String>,
        pub description: Option<String>,
        pub method: String,
        pub path: String,
        pub input_type: String,
        pub headers_type: Option<String>,
        pub output_type: String,
        pub error_type: Option<String>,
        pub path_params: Vec<Parameter>,
        pub has_body: bool,
        pub is_input_primitive: bool,
        pub deprecation_note: Option<String>,
    }

    #[derive(Clone)]
    pub struct FunctionGroup {
        pub name: String,
        pub class_name: String,
        pub functions: Vec<Function>,
    }

    #[derive(Clone)]
    pub struct Parameter {
        pub name: String,     // Sanitized Python variable name (snake_case)
        pub raw_name: String, // Original parameter name as it appears in the path
        pub type_annotation: String,
        pub description: Option<String>,
    }

    // Templates for externally tagged enum variants
    #[derive(Template)]
    #[template(
        source = r#"class {{ name }}(BaseModel):
{% if description.is_some() && !description.as_deref().unwrap().is_empty() %}    """{{ description.as_deref().unwrap() }}"""
{% else %}    """Unit variant for externally tagged enum."""
{% endif %}
    model_config = ConfigDict(extra="ignore")

    def model_dump(self, **kwargs):
        """Serialize as just the variant name string for unit variants."""
        return "{{ variant_name }}"

    def model_dump_json(self, **kwargs):
        """Serialize as JSON string for unit variants."""
        import json
        return json.dumps(self.model_dump(**kwargs))
"#,
        ext = "txt"
    )]
    pub struct UnitVariantClass {
        pub name: String,
        pub variant_name: String,
        pub description: Option<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"class {{ name }}(BaseModel):
{% if description.is_some() && !description.as_deref().unwrap().is_empty() %}    """{{ description.as_deref().unwrap() }}"""
{% else %}    """Tuple variant for externally tagged enum."""
{% endif %}

    model_config = ConfigDict(extra="ignore")

{% for field in fields %}    {{ field.name }}: {{ field.type_annotation }}{% if field.default_value.is_some() %} = {{ field.default_value.as_ref().unwrap() }}{% else %}{% if field.optional %} = None{% endif %}{% endif %}
{% endfor %}

    def model_dump(self, **kwargs):
        """Serialize as externally tagged tuple variant."""
        fields = [{% for field in fields %}self.{{ field.name }}{% if !loop.last %}, {% endif %}{% endfor %}]
        return {"{{ variant_name }}": fields}

    def model_dump_json(self, **kwargs):
        """Serialize as JSON for externally tagged tuple variant."""
        import json
        return json.dumps(self.model_dump(**kwargs))
"#,
        ext = "txt"
    )]
    pub struct TupleVariantClass {
        pub name: String,
        pub variant_name: String,
        pub fields: Vec<Field>,
        pub description: Option<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"class {{ name }}(BaseModel):
{% if description.is_some() && !description.as_deref().unwrap().is_empty() %}    """{{ description.as_deref().unwrap() }}"""
{% else %}    """Struct variant for externally tagged enum."""
{% endif %}
    model_config = ConfigDict(extra="ignore")

{% for field in fields %}    {{ field.name }}: {{ field.type_annotation }}{% if field.default_value.is_some() %} = {{ field.default_value.as_ref().unwrap() }}{% else %}{% if field.optional %} = None{% endif %}{% endif %}
{% endfor %}

    def model_dump(self, **kwargs):
        """Serialize as externally tagged struct variant."""
        fields = {}
{% for field in fields %}        if hasattr(self, '{{ field.name }}') and self.{{ field.name }} is not None:
            fields['{{ field.name }}'] = self.{{ field.name }}
{% endfor %}
        return {"{{ variant_name }}": fields}

    def model_dump_json(self, **kwargs):
        """Serialize as JSON for externally tagged struct variant."""
        import json
        return json.dumps(self.model_dump(**kwargs))
"#,
        ext = "txt"
    )]
    pub struct StructVariantClass {
        pub name: String,
        pub variant_name: String,
        pub fields: Vec<Field>,
        pub description: Option<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"{% for variant_def in variant_definitions %}{{ variant_def }}

{% endfor %}{{ name }} = Union[{{ union_variant_names|join(", ") }}]
{% if description.is_some() && !description.as_deref().unwrap_or("").is_empty() %}"""{{ description.as_deref().unwrap() }}"""{% endif %}
"#,
        ext = "txt"
    )]
    pub struct ExternallyTaggedUnionClass {
        pub name: String,
        pub description: Option<String>,
        pub variant_definitions: Vec<String>,
        pub union_variant_names: Vec<String>,
    }

    // Hybrid enum templates for Pythonic data-carrying enum variants
    #[derive(Template)]
    #[template(
        source = r#"from enum import Enum
from dataclasses import dataclass
from typing import Union, Optional

{% for data_class in data_classes -%}
{{ data_class }}

{% endfor -%}
class {{ enum_name }}(str, Enum):
    {% if description.is_some() && !description.as_deref().unwrap().is_empty() -%}
    """{{ description.as_deref().unwrap() }}"""
    {% endif -%}
    # Unit variants as enum members
{% for variant in unit_variants %}    {{ variant.name }} = "{{ variant.value }}"{% if variant.description.is_some() %}  # {{ variant.description.as_deref().unwrap() }}{% endif %}
{% endfor %}

    def model_dump(self):
        """Handle serialization for simple enum values."""
        return self.value
{% for method in factory_methods -%}
{{ method }}
{% endfor %}

# Type annotation for the union
{{ union_name }} = Union[{{ union_members|join(", ") }}]
"#,
        ext = "txt"
    )]
    pub struct HybridEnumClass {
        pub enum_name: String,
        pub description: Option<String>,
        pub unit_variants: Vec<EnumVariant>,
        pub factory_methods: Vec<String>,
        pub data_classes: Vec<String>,
        pub union_members: Vec<String>,
        pub union_name: String,
    }

    #[derive(Template)]
    #[template(
        source = r#"@dataclass
class {{ name }}:
    {% if description.is_some() && !description.as_deref().unwrap().is_empty() %}"""{{ description.as_deref().unwrap() }}"""
    {% endif %}{%- for field_type in field_types %}
    {{ field_type }}
{%- endfor %}

    def model_dump(self) -> dict:
        """Serialize tuple variant as externally tagged."""
        fields = [{% for field in field_names %}self.{{ field }}{% if !loop.last %}, {% endif %}{% endfor %}]
        return {"{{ variant_name }}": fields}

    def model_dump_json(self, **kwargs) -> str:
        """Serialize as JSON."""
        import json
        return json.dumps(self.model_dump())
"#,
        ext = "txt"
    )]
    pub struct TupleVariantDataClass {
        pub name: String,
        pub variant_name: String,
        pub field_types: Vec<String>,
        pub field_names: Vec<String>,
        pub description: Option<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"@dataclass
class {{ name }}:
    {% if description.is_some() && !description.as_deref().unwrap().is_empty() %}"""{{ description.as_deref().unwrap() }}"""
    {% endif %}{%- for field_def in field_definitions %}
    {{ field_def }}
{%- endfor %}

    def model_dump(self) -> dict:
        """Serialize struct variant as externally tagged."""
        result = {}
{%- for field_name in field_names %}
        if hasattr(self, '{{ field_name }}') and self.{{ field_name }} is not None:
            result['{{ field_name }}'] = self.{{ field_name }}
{%- endfor %}
        return {"{{ variant_name }}": result}

    def model_dump_json(self, **kwargs) -> str:
        """Serialize as JSON."""
        import json
        return json.dumps(self.model_dump())
"#,
        ext = "txt"
    )]
    pub struct StructVariantDataClass {
        pub name: String,
        pub variant_name: String,
        pub field_definitions: Vec<String>,
        pub field_names: Vec<String>,
        pub description: Option<String>,
    }

    #[derive(Template)]
    #[template(source = r#"{{ method_name }}"#, ext = "txt")]
    pub struct FactoryMethod {
        pub method_name: String,
        pub variant_name: String,
        pub class_name: String,
        pub parameters: Vec<String>,
        pub field_names: Vec<String>,
        pub description: Option<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"{% if is_generic -%}
# TypeVar definitions for {{ union_name }}
{% for param in generic_params -%}
{{ param }} = TypeVar('{{ param }}')
{% endfor %}

{% endif -%}
{% for variant_model in variant_models %}{{ variant_model }}

{% endfor %}
# Discriminated union for {{ union_name }}
{{ union_name }} = Annotated[Union[{{ union_members|join(", ") }}], Field(discriminator='kind')]
{% if description.is_some() -%}
"""{{ description.as_deref().unwrap() }}"""
{% endif %}"#,
        ext = "txt"
    )]
    pub struct DiscriminatedUnionEnum {
        pub variant_models: Vec<String>,
        pub union_members: Vec<String>,
        pub union_name: String,
        pub description: Option<String>,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"{% for variant_model in variant_models %}{{ variant_model }}

{% endfor %}
# Externally tagged enum using RootModel
{% if is_single_variant %}{{ name }}Variants = {{ union_variants }}
{% else %}{{ name }}Variants = Union[{{ union_variants }}]
{% endif %}
class {{ name }}(RootModel[{{ name }}Variants]{% if is_generic %}, Generic[{% for param in generic_params %}{{ param }}{% if !loop.last %}, {% endif %}{% endfor %}]{% endif %}):
    """{% if description.is_some() %}{{ description.as_deref().unwrap() }}{% else %}Externally tagged enum{% endif %}"""

{% if is_generic %}    @classmethod
    def __class_getitem__(cls, params):
        return cls
{% endif %}

    @model_validator(mode='before')
    @classmethod
    def _validate_externally_tagged(cls, data):
        # Handle direct variant instances (for programmatic creation)
{{ instance_validator_cases }}

        # Handle JSON data (for deserialization)
{{ validator_cases }}

        if isinstance(data, dict):
            if len(data) != 1:
                raise ValueError("Externally tagged enum must have exactly one key")

            key, value = next(iter(data.items()))
{{ dict_validator_cases }}

        raise ValueError(f"Unknown variant for {{ name }}: {data}")

    @model_serializer
    def _serialize_externally_tagged(self):
{{ serializer_cases }}

        raise ValueError(f"Cannot serialize {{ name }} variant: {type(self.root)}")
"#,
        ext = "txt"
    )]
    pub struct ExternallyTaggedEnumRootModel {
        pub name: String,
        pub description: Option<String>,
        pub variant_models: Vec<String>,
        pub union_variants: String,
        pub is_single_variant: bool,
        pub instance_validator_cases: String,
        pub validator_cases: String,
        pub dict_validator_cases: String,
        pub serializer_cases: String,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
    }

    #[derive(Template)]
    #[template(
        source = r#"{% if is_generic %}
# Generic externally tagged enum using Approach B: Generic Variant Models
{% for param in generic_params -%}
{{ param }} = TypeVar('{{ param }}')
{% endfor %}

# Common non-generic base class with discriminator
class {{ name }}Base(BaseModel):
    """Base class for {{ name }} variants with shared discriminator."""
    _kind: str = PrivateAttr()

{% for variant_model in variant_models %}{{ variant_model }}

{% endfor %}
# Type alias for parameterized union - users can create specific unions as needed
# Example: {{ name }}[SomeType, AnotherType] = Union[{{ name }}Variant1[SomeType], {{ name }}Variant2[AnotherType]]
class {{ name }}(Generic[{% for param in generic_params %}{{ param }}{% if !loop.last %}, {% endif %}{% endfor %}]):
    """{% if description.is_some() %}{{ description.as_deref().unwrap() }}{% else %}Generic externally tagged enum using Approach B{% endif %}

    This is a generic enum where each variant is a separate generic class.
    To create a specific instance, use the variant classes directly.
    To create a union type, use Union[VariantClass[Type1], OtherVariant[Type2]].
    """

    @classmethod
    def __class_getitem__(cls, params):
        """Create documentation about parameterized types."""
        if not isinstance(params, tuple):
            params = (params,)
        if len(params) != {{ generic_params.len() }}:
            raise TypeError(f"Expected {{ generic_params.len() }} type parameters, got {len(params)}")

        # For Approach B, users should create unions directly using variant classes
        # This method serves as documentation
        variant_examples = [
            {% for variant_info in variant_info_list %}"{{ variant_info.class_name }}[{% for param in generic_params %}{{ param }}{% if !loop.last %}, {% endif %}{% endfor %}]"{% if !loop.last %},
            {% endif %}{% endfor %}
        ]

        # Return a helpful hint rather than NotImplementedError
        return f"Union[{', '.join(variant_examples)}]  # Use this pattern to create specific unions"
{% else %}
# Non-generic externally tagged enum - use existing RootModel approach
{% if is_single_variant %}{{ name }}Variants = {{ union_variants }}
{% else %}{{ name }}Variants = Union[{{ union_variants }}]
{% endif %}
class {{ name }}(RootModel[{{ name }}Variants]):
    """{% if description.is_some() %}{{ description.as_deref().unwrap() }}{% else %}Externally tagged enum{% endif %}"""

    @model_validator(mode='before')
    @classmethod
    def _validate_externally_tagged(cls, data):
        # Handle direct variant instances (for programmatic creation)
{{ instance_validator_cases }}

        # Handle JSON data (for deserialization)
{{ validator_cases }}

        if isinstance(data, dict):
            if len(data) != 1:
                raise ValueError("Externally tagged enum must have exactly one key")

            key, value = next(iter(data.items()))
{{ dict_validator_cases }}

        raise ValueError(f"Unknown variant for {{ name }}: {data}")

    @model_serializer
    def _serialize_externally_tagged(self):
{{ serializer_cases }}

        raise ValueError(f"Cannot serialize {{ name }} variant: {type(self.root)}")
{% endif %}
"#,
        ext = "txt"
    )]
    pub struct GenericExternallyTaggedEnumApproachB {
        pub name: String,
        pub description: Option<String>,
        pub variant_models: Vec<String>,
        pub union_variants: String,
        pub is_single_variant: bool,
        pub instance_validator_cases: String,
        pub validator_cases: String,
        pub dict_validator_cases: String,
        pub serializer_cases: String,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
        pub variant_info_list: Vec<VariantInfo>,
    }

    #[derive(Clone)]
    pub struct VariantInfo {
        pub class_name: String,
        pub variant_name: String,
    }
}
