use std::collections::HashMap;

use anyhow::Context;
use askama::Template;

use crate::{Schema, TypeReference};
use reflectapi_schema::{Function, Type};

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

/// Generate Python client code from a schema
pub fn generate(mut schema: Schema, config: &Config) -> anyhow::Result<String> {
    let implemented_types = build_implemented_types();

    // Consolidate types to avoid duplicates
    schema.consolidate_types();

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

    // Check if we need ReflectapiOption import
    let has_reflectapi_option = schema_uses_reflectapi_option(&schema, &all_type_names);

    // Generate imports
    let imports = templates::Imports {
        has_async: config.generate_async,
        has_sync: config.generate_sync,
        has_testing: config.generate_testing,
        has_enums,
        has_reflectapi_option,
    };
    generated_code.push(imports.render().context("Failed to render imports")?);

    // Types that are provided by the runtime library and should not be generated
    let runtime_provided_types = ["reflectapi::Option"];

    // Render all types (models and enums)
    let mut rendered_types = HashMap::new();
    for original_type_name in all_type_names {
        // Skip types provided by the runtime
        if runtime_provided_types.contains(&original_type_name.as_str()) {
            continue;
        }

        let type_def = schema.get_type(&original_type_name).unwrap();
        let name = improve_class_name(type_def.name());
        let rendered = render_type(type_def, &schema, &implemented_types)?;
        // Only store non-empty renders (excludes unwrapped tuple structs)
        if !rendered.trim().is_empty() {
            rendered_types.insert(name.clone(), rendered.clone());
            generated_code.push(rendered);
        }
    }

    // Generate client class with nested method organization
    let functions_by_name: HashMap<String, &Function> =
        schema.functions().map(|f| (f.name.clone(), f)).collect();

    // Group functions by their prefix and separate top-level functions
    let mut function_groups: HashMap<String, Vec<templates::Function>> = HashMap::new();
    let mut top_level_functions: Vec<templates::Function> = Vec::new();

    for function_schema in functions_by_name.values() {
        let rendered_function = render_function(function_schema, &schema, &implemented_types)?;

        // Check for grouping patterns: underscore or dot notation
        if let Some(separator_pos) = function_schema.name.find('_').or_else(|| function_schema.name.find('.')) {
            let group_name = &function_schema.name[..separator_pos];
            let method_name = &function_schema.name[separator_pos + 1..];

            // Create a modified function with the shortened name for nested access
            let mut nested_function = rendered_function.clone();
            nested_function.name = method_name.to_string();
            nested_function.original_name = Some(rendered_function.name.clone());

            function_groups
                .entry(group_name.to_string())
                .or_default()
                .push(nested_function);
        } else {
            // Functions without separators remain as top-level methods
            top_level_functions.push(rendered_function);
        }
    }

    // Convert function groups to structured format
    let structured_function_groups: Vec<templates::FunctionGroup> = function_groups
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

    // Generate testing utilities if requested
    if config.generate_testing {
        // Only include user-defined types, not primitive types that are mapped to built-ins
        let user_defined_types: Vec<String> = rendered_types
            .keys()
            .filter(|name| {
                // Filter out types that are just mapped to primitives/built-ins
                !name.starts_with("std::")
                    && !matches!(
                        name.as_str(),
                        "f64"
                            | "u8"
                            | "i32"
                            | "bool"
                            | "String"
                            | "ChronoDateTime"
                            | "StdVecVec"
                            | "StdStringString"
                            | "StdTupleTuple0"
                            | "U8"
                            | "F64"
                    )
            })
            .cloned()
            .collect();

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

fn render_type(
    type_def: &Type,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<String> {
    match type_def {
        Type::Struct(s) => render_struct(s, schema, implemented_types),
        Type::Enum(e) => render_enum(e, schema, implemented_types),
        Type::Primitive(_p) => {
            // Primitive types are handled by implemented_types mapping
            Ok(String::new()) // This shouldn't be reached normally
        }
    }
}

fn render_struct(
    struct_def: &reflectapi_schema::Struct,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<String> {
    // Check if this is a single-field tuple struct that should be unwrapped
    if struct_def.is_tuple() && struct_def.fields.len() == 1 {
        // Skip generation for single-field tuple structs
        // They should be unwrapped and used directly
        return Ok(String::new());
    }

    let fields = struct_def
        .fields
        .iter()
        .map(|field| {
            let field_type = type_ref_to_python_type(&field.type_ref, schema, implemented_types)?;

            // Fix default value handling for optional fields
            let (optional, default_value) = if !field.required {
                (true, Some("None".to_string()))
            } else {
                (false, None)
            };

            Ok(templates::Field {
                name: sanitize_field_name(field.name()),
                type_annotation: field_type,
                description: Some(field.description().to_string()),
                deprecation_note: field.deprecation_note.clone(),
                optional,
                default_value,
            })
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    // Check if this is a generic struct (has type parameters)
    let has_generics = !struct_def.parameters.is_empty();
    let class_name = improve_class_name(&struct_def.name);

    let class_template = templates::DataClass {
        name: class_name,
        description: Some(struct_def.description().to_string()),
        fields,
        is_tuple: false,
        is_generic: has_generics,
    };

    class_template.render().context("Failed to render struct")
}

fn render_enum(
    enum_def: &reflectapi_schema::Enum,
    _schema: &Schema,
    _implemented_types: &HashMap<String, String>,
) -> anyhow::Result<String> {
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

    enum_template.render().context("Failed to render enum")
}

// OneOf types don't exist in the current schema - this was removed

fn render_function(
    function: &Function,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<templates::Function> {
    let input_type = if let Some(input_type) = function.input_type.as_ref() {
        type_ref_to_python_type(input_type, schema, implemented_types)?
    } else {
        "None".to_string()
    };

    let output_type = if let Some(output_type) = function.output_type.as_ref() {
        type_ref_to_python_type(output_type, schema, implemented_types)?
    } else {
        "Any".to_string()
    };

    let error_type = if let Some(error_type) = function.error_type.as_ref() {
        Some(type_ref_to_python_type(
            error_type,
            schema,
            implemented_types,
        )?)
    } else {
        None
    };

    // Extract path and query parameters from input type
    let (path_params, query_params) = extract_parameters(schema, function, &input_type)?;

    // Use function name as path if path is empty (like TypeScript generator does)
    let path = if function.path.is_empty() {
        format!("/{}", function.name)
    } else {
        function.path.clone()
    };

    // Check if input type is a primitive type
    let is_input_primitive = if let Some(input_type_ref) = &function.input_type {
        is_primitive_type(&input_type_ref.name)
    } else {
        false
    };

    // Determine if we need a body parameter
    // For GET requests, body is not needed as all parameters become path/query params
    // For POST requests, we use the body if there are fields not covered by path params
    let has_body = if function.readonly {
        // GET requests don't have bodies
        false
    } else {
        // POST requests have bodies if there's an input type
        function.input_type.is_some()
    };

    Ok(templates::Function {
        name: to_snake_case(&function.name),
        original_name: None, // Will be set later if this function is nested
        description: Some(function.description().to_string()),
        method: if function.readonly {
            "GET".to_string()
        } else {
            "POST".to_string()
        },
        path,
        input_type,
        output_type,
        error_type,
        path_params,
        query_params,
        has_body,
        is_input_primitive,
        deprecation_note: function.deprecation_note.clone(),
    })
}

// Extract path and query parameters from function definition
fn extract_parameters(
    schema: &Schema,
    function: &Function,
    _input_type: &str,
) -> anyhow::Result<(Vec<templates::Parameter>, Vec<templates::Parameter>)> {
    let mut path_params = Vec::new();
    let mut query_params = Vec::new();

    // Extract path parameters by finding {param_name} patterns in the path
    let path = &function.path;
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

    // Extract query parameters from input type if it's a GET request
    if function.readonly && function.input_type.is_some() {
        if let Some(input_type_ref) = &function.input_type {
            // Try to get the struct definition from the schema
            if let Some(reflectapi_schema::Type::Struct(struct_def)) =
                schema.get_type(&input_type_ref.name)
            {
                // Get path parameter names to exclude them from query params
                let path_param_names: std::collections::HashSet<String> =
                    path_params.iter().map(|p| p.name.clone()).collect();

                // For GET requests, fields not in path become query parameters
                for field in struct_def.fields.iter() {
                    let field_name = to_snake_case(field.name());

                    // Skip if this field is already a path parameter
                    if !path_param_names.contains(&field_name) {
                        let field_type = type_ref_to_python_type(
                            &field.type_ref,
                            schema,
                            &build_implemented_types(),
                        )?;
                        query_params.push(templates::Parameter {
                            name: field_name,
                            type_annotation: field_type,
                            description: Some(format!("Query parameter: {}", field.name())),
                        });
                    }
                }
            }
        }
    }

    Ok((path_params, query_params))
}

// Check if a type name represents a primitive type
fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
        | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
        | "f32" | "f64"
        | "bool"
        | "String" | "str"
        | "char"
    )
}

// Utility functions for string case conversion
fn to_snake_case(s: &str) -> String {
    // First, replace dots and hyphens with underscores to handle function names like "pets.list" and "get-first"
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

/// Improve Python class names by removing redundant prefixes and suffixes
fn improve_class_name(original_name: &str) -> String {
    let pascal_name = to_pascal_case(original_name);

    // Define transformation rules as patterns
    let transformations = [
        // Remove common module prefixes
        ("MyapiModel", ""),
        ("MyapiProto", ""),
        ("StdOption", ""),
        ("StdVec", ""),
        ("StdResult", "Result"),
        ("StdTuple", "Tuple"),
        ("StdCollectionsHashMap", "HashMap"),
        ("StdCollectionsBTreeMap", "BTreeMap"),
        ("ReflectapiOption", "Option"),
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

    // Remove redundant suffixes (e.g., "OptionOption" -> "Option")
    let redundant_suffixes = [
        ("OptionOption", "Option"),
        ("VecVec", "Vec"),
        ("ResultResult", "Result"),
        ("StringString", "String"),
    ];

    for (suffix, replacement) in &redundant_suffixes {
        if improved.ends_with(suffix) {
            let prefix_len = improved.len() - suffix.len();
            improved = format!("{}{}", &improved[..prefix_len], replacement);
            break;
        }
    }

    // Apply domain-specific improvements
    let domain_improvements = [
        ("Kind", "PetKind"),
        ("InputPet", "Pet"),
        ("OutputPet", "PetDetails"),
        ("Input", "Request"),
        ("Output", "Response"),
        ("Error", "Error"),
        ("Paginated", "Paginated"),
        ("Headers", "Headers"),
        ("Metadata", "Metadata"),
    ];

    for (pattern, replacement) in &domain_improvements {
        if improved == *pattern {
            return replacement.to_string();
        }
    }

    // Handle prefix preservation for certain patterns
    if improved.starts_with("Pets") || improved.starts_with("User") || improved.starts_with("Api") {
        return improved;
    }

    // Clean up any remaining double patterns
    improved = improved.replace("__", "_").replace('_', "");

    // Ensure it starts with uppercase (proper Pascal case)
    if let Some(first_char) = improved.chars().next() {
        if first_char.is_lowercase() {
            let mut chars = improved.chars();
            chars.next();
            improved = format!("{}{}", first_char.to_uppercase(), chars.collect::<String>());
        }
    }

    improved
}

fn to_screaming_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

fn sanitize_field_name(s: &str) -> String {
    let snake_case = to_snake_case(s);

    // If the field name starts with a digit, prefix it with "field_"
    if snake_case
        .chars()
        .next()
        .map_or(false, |c| c.is_ascii_digit())
    {
        format!("field_{}", snake_case)
    } else {
        snake_case
    }
}

// Type substitution function - handles TypeReference to Python type conversion
fn type_ref_to_python_type(
    type_ref: &TypeReference,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<String> {
    // Check if this type has a direct mapping
    if let Some(python_type) = implemented_types.get(&type_ref.name) {
        if type_ref.arguments.is_empty() {
            return Ok(python_type.clone());
        }

        // Handle generic types with arguments
        let mut result = python_type.clone();

        // Get type parameter names for substitution (e.g., "T", "K", "V")
        let type_params = get_type_parameters(&type_ref.name);

        for (param, arg) in type_params.iter().zip(type_ref.arguments.iter()) {
            let resolved_arg = type_ref_to_python_type(arg, schema, implemented_types)?;
            result = result.replace(param, &resolved_arg);
        }

        return Ok(result);
    }

    // Check if it's a user-defined type in the schema
    if let Some(type_def) = schema.get_type(&type_ref.name) {
        // Check if this is a single-field tuple struct that should be unwrapped
        if let reflectapi_schema::Type::Struct(struct_def) = type_def {
            if struct_def.is_tuple() && struct_def.fields.len() == 1 {
                // Return the inner type instead of the wrapper
                let inner_field = &struct_def.fields[0];
                return type_ref_to_python_type(&inner_field.type_ref, schema, implemented_types);
            }
        }
        return Ok(improve_class_name(&type_ref.name));
    }

    // Fallback - use the original name
    Ok(improve_class_name(&type_ref.name))
}

// Note: substitute_type function removed as it's no longer needed

// Get type parameter names for a given type
fn get_type_parameters(type_name: &str) -> Vec<&'static str> {
    match type_name {
        "std::vec::Vec" => vec!["T"],
        "std::option::Option" => vec!["T"],
        "reflectapi::Option" => vec!["T"],
        "std::collections::HashMap" => vec!["K", "V"],
        "std::collections::BTreeMap" => vec!["K", "V"],
        "std::result::Result" => vec!["T", "E"],
        _ => vec![],
    }
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

fn build_implemented_types() -> HashMap<String, String> {
    let mut types = HashMap::new();

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

    // Date/time types
    types.insert("chrono::DateTime".to_string(), "datetime".to_string());
    types.insert("chrono::NaiveDateTime".to_string(), "datetime".to_string());
    types.insert("chrono::NaiveDate".to_string(), "date".to_string());
    types.insert("uuid::Uuid".to_string(), "UUID".to_string());

    // Special tuple/unit types
    types.insert("std::tuple::Tuple0".to_string(), "None".to_string());

    // Common serde types
    types.insert("serde_json::Value".to_string(), "Any".to_string());

    types
}

// Note: format_default_value function removed as it's no longer needed

fn format_python_code(code: &str) -> anyhow::Result<String> {
    // Try to format with black if available
    use std::process::{Command, Stdio};

    let child = Command::new("black")
        .arg("--code")
        .arg(code)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    match child {
        Ok(process) => {
            let output = process.wait_with_output()?;
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                // Fall back to original code if black fails
                Ok(code.to_string())
            }
        }
        Err(_) => {
            // black not available, return original code
            Ok(code.to_string())
        }
    }
}

pub mod templates {
    use super::*;

    #[derive(Template)]
    #[template(
        source = r#"""""
Generated Python client for {{ package_name }}.

DO NOT MODIFY THIS FILE MANUALLY.
This file is automatically generated by ReflectAPI.
"""

from __future__ import annotations
"#,
        ext = "txt"
    )]
    pub struct FileHeader {
        pub package_name: String,
    }

    #[derive(Template)]
    #[template(
        source = r#"from __future__ import annotations

from datetime import datetime
from typing import Any, Optional, TypeVar, Generic
import warnings
{% if has_enums %}
from enum import Enum
{% endif %}

from pydantic import BaseModel, ConfigDict
{% if has_async %}
from reflectapi_runtime import AsyncClientBase, ApiResponse
{% endif %}
{% if has_sync %}
from reflectapi_runtime import ClientBase
{% endif %}
{% if has_testing %}
from reflectapi_runtime.testing import MockClient, create_api_response
{% endif %}
{% if has_reflectapi_option %}
from reflectapi_runtime import ReflectapiOption
{% endif %}

T = TypeVar('T')
"#,
        ext = "txt"
    )]
    pub struct Imports {
        pub has_async: bool,
        pub has_sync: bool,
        pub has_testing: bool,
        pub has_enums: bool,
        pub has_reflectapi_option: bool,
    }

    #[derive(Template)]
    #[template(
        source = r#"class {{ name }}(BaseModel{% if is_generic %}, Generic[T]{% endif %}):
{% if description.is_some() && !description.as_deref().unwrap().is_empty() %}    """{{ description.as_deref().unwrap() }}"""
{% endif %}    
    model_config = ConfigDict(extra="ignore")
    
{% for field in fields %}
    {{ field.name }}: {{ field.type_annotation }}{% if field.optional %} = None{% else %}{% if field.default_value.is_some() %} = {{ field.default_value.as_ref().unwrap() }}{% endif %}{% endif %}
{% if field.description.is_some() && !field.description.as_deref().unwrap().is_empty() %}    """{{ field.description.as_deref().unwrap() }}{% if field.deprecation_note.is_some() %}
    
    .. deprecated::
       {{ field.deprecation_note.as_deref().unwrap() }}{% endif %}"""
{% endif %}{% endfor %}
"#,
        ext = "txt"
    )]
    pub struct DataClass {
        pub name: String,
        pub description: Option<String>,
        pub fields: Vec<Field>,
        pub is_tuple: bool,
        pub is_generic: bool,
    }

    #[derive(Template)]
    #[template(
        source = r#"class {{ name }}(str, Enum):
{% if description.is_some() && !description.as_deref().unwrap().is_empty() %}    """{{ description.as_deref().unwrap() }}
    
    Attributes:
{% for variant in variants %}        {{ variant.name }}: {{ variant.description.as_deref().unwrap_or("") }}
{% endfor %}    """
{% endif %}    
{% for variant in variants %}
    {{ variant.name }} = "{{ variant.value }}"
{% if variant.description.is_some() && !variant.description.as_deref().unwrap().is_empty() %}    """{{ variant.description.as_deref().unwrap() }}"""
{% endif %}{% endfor %}
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
        source = r#"from typing import Union

{{ name }} = Union[{% for variant in variants %}{{ variant.type_annotation }}{% if !loop.last %}, {% endif %}{% endfor %}]
"""{{ description.as_deref().unwrap_or("") }}"""
"#,
        ext = "txt"
    )]
    pub struct UnionClass {
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
{% for param in function.path_params %}
            {{ param.name }}: {{ param.type_annotation }},
{% endfor %}
{% for param in function.query_params %}
            {{ param.name }}: Optional[{{ param.type_annotation }}] = None,
{% endfor %}
{% if function.has_body %}
            data: Optional[{{ function.input_type }}] = None,
{% endif %}
        ) -> ApiResponse[{{ function.output_type }}]:
            """{{ function.description.as_deref().unwrap_or("") }}{% if function.has_body %}
            
            Args:
                data: Request data for the {{ function.name }} operation.{% endif %}{% if !function.path_params.is_empty() %}
{% for param in function.path_params %}                {{ param.name }}: {{ param.description.as_deref().unwrap_or("Path parameter") }}
{% endfor %}{% endif %}{% if !function.query_params.is_empty() %}
{% for param in function.query_params %}                {{ param.name }}: {{ param.description.as_deref().unwrap_or("Query parameter") }} (optional)
{% endfor %}{% endif %}
            
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
            {% endif %}
            path = "{{ function.path }}"
{% for param in function.path_params %}
            path = path.replace("{" + "{{ param.name }}" + "}", str({{ param.name }}))
{% endfor %}
            
            params: dict[str, Any] = {}
{% for param in function.query_params %}
            if {{ param.name }} is not None:
                params["{{ param.name }}"] = {{ param.name }}
{% endfor %}
            
            return await self._client._make_request(
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
                response_model={{ function.output_type }},
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
{% for param in function.path_params %}
        {{ param.name }}: {{ param.type_annotation }},
{% endfor %}
{% for param in function.query_params %}
        {{ param.name }}: Optional[{{ param.type_annotation }}] = None,
{% endfor %}
{% if function.has_body %}
        data: Optional[{{ function.input_type }}] = None,
{% endif %}
    ) -> ApiResponse[{{ function.output_type }}]:
        """{{ function.description.as_deref().unwrap_or("") }}{% if function.has_body %}
        
        Args:
            data: Request data for the {{ function.name }} operation.{% endif %}{% if !function.path_params.is_empty() %}
{% for param in function.path_params %}            {{ param.name }}: {{ param.description.as_deref().unwrap_or("Path parameter") }}
{% endfor %}{% endif %}{% if !function.query_params.is_empty() %}
{% for param in function.query_params %}            {{ param.name }}: {{ param.description.as_deref().unwrap_or("Query parameter") }} (optional)
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
{% for param in function.path_params %}
        path = path.replace("{" + "{{ param.name }}" + "}", str({{ param.name }}))
{% endfor %}
        
        params: dict[str, Any] = {}
{% for param in function.query_params %}
        if {{ param.name }} is not None:
            params["{{ param.name }}"] = {{ param.name }}
{% endfor %}
        
        return await self._make_request(
            "{{ function.method }}",
            path,
            params=params if params else None,
{% if function.has_body %}
{% if function.is_input_primitive %}
            json_data=data,
{% else %}
            json_data=data.model_dump() if data else None,
{% endif %}
{% endif %}
            response_model={{ function.output_type }},
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
{% for param in function.path_params %}
            {{ param.name }}: {{ param.type_annotation }},
{% endfor %}
{% for param in function.query_params %}
            {{ param.name }}: Optional[{{ param.type_annotation }}] = None,
{% endfor %}
{% if function.has_body %}
            data: Optional[{{ function.input_type }}] = None,
{% endif %}
        ) -> ApiResponse[{{ function.output_type }}]:
            """{{ function.description.as_deref().unwrap_or("") }}{% if function.has_body %}
            
            Args:
                data: Request data for the {{ function.name }} operation.{% endif %}{% if !function.path_params.is_empty() %}
{% for param in function.path_params %}                {{ param.name }}: {{ param.description.as_deref().unwrap_or("Path parameter") }}
{% endfor %}{% endif %}{% if !function.query_params.is_empty() %}
{% for param in function.query_params %}                {{ param.name }}: {{ param.description.as_deref().unwrap_or("Query parameter") }} (optional)
{% endfor %}{% endif %}
            
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
            {% endif %}
            path = "{{ function.path }}"
{% for param in function.path_params %}
            path = path.replace("{" + "{{ param.name }}" + "}", str({{ param.name }}))
{% endfor %}
            
            params: dict[str, Any] = {}
{% for param in function.query_params %}
            if {{ param.name }} is not None:
                params["{{ param.name }}"] = {{ param.name }}
{% endfor %}
            
            return self._client._make_request(
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
                response_model={{ function.output_type }},
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
{% for param in function.path_params %}
        {{ param.name }}: {{ param.type_annotation }},
{% endfor %}
{% for param in function.query_params %}
        {{ param.name }}: Optional[{{ param.type_annotation }}] = None,
{% endfor %}
{% if function.has_body %}
        data: Optional[{{ function.input_type }}] = None,
{% endif %}
    ) -> ApiResponse[{{ function.output_type }}]:
        """{{ function.description.as_deref().unwrap_or("") }}{% if function.has_body %}
        
        Args:
            data: Request data for the {{ function.name }} operation.{% endif %}{% if !function.path_params.is_empty() %}
{% for param in function.path_params %}            {{ param.name }}: {{ param.description.as_deref().unwrap_or("Path parameter") }}
{% endfor %}{% endif %}{% if !function.query_params.is_empty() %}
{% for param in function.query_params %}            {{ param.name }}: {{ param.description.as_deref().unwrap_or("Query parameter") }} (optional)
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
{% for param in function.path_params %}
        path = path.replace("{" + "{{ param.name }}" + "}", str({{ param.name }}))
{% endfor %}
        
        params: dict[str, Any] = {}
{% for param in function.query_params %}
        if {{ param.name }} is not None:
            params["{{ param.name }}"] = {{ param.name }}
{% endfor %}
        
        return self._make_request(
            "{{ function.method }}",
            path,
            params=params if params else None,
{% if function.has_body %}
{% if function.is_input_primitive %}
            json_data=data,
{% else %}
            json_data=data.model_dump() if data else None,
{% endif %}
{% endif %}
            response_model={{ function.output_type }},
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
    }

    pub struct EnumVariant {
        pub name: String,
        pub value: String,
        pub description: Option<String>,
    }

    pub struct UnionVariant {
        pub name: String,
        pub type_annotation: String,
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
        pub output_type: String,
        pub error_type: Option<String>,
        pub path_params: Vec<Parameter>,
        pub query_params: Vec<Parameter>,
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
        pub name: String,
        pub type_annotation: String,
        pub description: Option<String>,
    }
}
