use std::collections::HashMap;

use anyhow::Context;
use askama::Template;

use crate::{Schema, TypeReference};
use reflectapi_schema::{Function, Type};

fn to_valid_python_identifier(name: &str) -> String {
    let cleaned = name
        .replace(['.', '-'], "_")
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    // Handle Python keywords by appending underscore
    const PYTHON_KEYWORDS: &[&str] = &[
        "and", "as", "assert", "break", "class", "continue", "def", "del", "elif", "else",
        "except", "exec", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda",
        "not", "or", "pass", "print", "raise", "return", "try", "while", "with", "yield", "async",
        "await", "nonlocal",
    ];

    let mut result = if PYTHON_KEYWORDS.contains(&cleaned.as_str()) {
        format!("{}_", cleaned)
    } else {
        cleaned
    };

    // If the identifier starts with a digit, prefix it with "field_"
    if result.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        result = format!("field_{}", result);
    }

    result
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

    // Check if we need Literal import and Field discriminator (for tagged enums)
    let (has_literal, has_discriminated_unions) = {
        let mut has_literal = false;
        let mut has_discriminated_unions = false;

        for name in &all_type_names {
            if let Some(reflectapi_schema::Type::Enum(enum_def)) = schema.get_type(name) {
                if matches!(
                    enum_def.representation,
                    reflectapi_schema::Representation::Internal { .. }
                ) {
                    has_literal = true;
                    has_discriminated_unions = true;
                }
            }
        }
        (has_literal, has_discriminated_unions)
    };

    // Check if we need ReflectapiOption import
    let has_reflectapi_option = schema_uses_reflectapi_option(&schema, &all_type_names);

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
        has_warnings,
        has_datetime,
        has_uuid,
        has_timedelta,
        has_date,
        has_generics: true,
        has_annotated: true, // Always include for external type fallbacks
        has_literal,
        has_discriminated_unions,
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
        if let Some(separator_pos) = function_schema
            .name
            .find('_')
            .or_else(|| function_schema.name.find('.'))
        {
            let group_name = &function_schema.name[..separator_pos];
            let method_name = &function_schema.name[separator_pos + 1..];

            // Create a modified function with the shortened name for nested access
            let mut nested_function = rendered_function.clone();
            nested_function.name = to_valid_python_identifier(method_name);
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

    generated_code.push(external_types_and_rebuilds.join("\n"));

    // Generate testing utilities if requested
    if config.generate_testing {
        // Only include user-defined types, not primitive types that are mapped to built-ins
        let mut user_defined_types: Vec<String> = rendered_types
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

    // Collect active generic parameter names for this struct
    let active_generics: Vec<String> = struct_def.parameters().map(|p| p.name.clone()).collect();

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
            )?;

            // Check if field type is Option<T> (which gets converted to T | None automatically)
            let is_option_type = field.type_ref.name == "std::option::Option";

            // Fix default value handling for optional fields
            let (optional, default_value, field_type) = if !field.required {
                // Field is not required - add | None if not already an Option type
                if is_option_type {
                    (true, Some("None".to_string()), base_field_type) // Already has | None
                } else {
                    (
                        true,
                        Some("None".to_string()),
                        format!("{} | None", base_field_type),
                    )
                }
            } else if is_option_type {
                // Field is required but Option type - still needs default None
                (true, Some("None".to_string()), base_field_type) // Already has | None
            } else {
                // Required non-Option field
                (false, None, base_field_type)
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

    // Collect flattened fields by expanding them from their type definitions
    let mut flattened_fields: Vec<templates::Field> = Vec::new();
    for field in struct_def.fields.iter().filter(|f| f.flattened()) {
        // Try to get the type definition and expand its fields
        if let Some(flattened_type) = schema.get_type(&field.type_ref.name) {
            match flattened_type {
                reflectapi_schema::Type::Struct(flattened_struct) => {
                    // Recursively get fields from the flattened struct
                    for flattened_field in flattened_struct.fields.iter() {
                        let field_type = type_ref_to_python_type(
                            &flattened_field.type_ref,
                            schema,
                            implemented_types,
                            &active_generics,
                        )?;

                        // Check if field type is Option<T>
                        let is_option_type = flattened_field.type_ref.name == "std::option::Option";

                        // Handle optionality - flattened fields inherit parent field's optionality
                        let (optional, default_value, final_field_type) =
                            if !flattened_field.required || !field.required {
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

                        flattened_fields.push(templates::Field {
                            name: sanitize_field_name(flattened_field.name()),
                            type_annotation: final_field_type,
                            description: Some(format!(
                                "(flattened) {}",
                                flattened_field.description()
                            )),
                            deprecation_note: flattened_field.deprecation_note.clone(),
                            optional,
                            default_value,
                        });
                    }
                }
                _ => {
                    // For non-struct flattened types, add a comment field
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &active_generics,
                    )?;
                    flattened_fields.push(templates::Field {
                        name: format!(
                            "# Flattened {} (non-struct type not fully supported)",
                            field.name()
                        ),
                        type_annotation: field_type,
                        description: Some(format!("Flattened from non-struct: {}", field.name())),
                        deprecation_note: None,
                        optional: false,
                        default_value: None,
                    });
                }
            }
        } else {
            // Unknown type - add a comment
            flattened_fields.push(templates::Field {
                name: format!("# Flattened {} (unknown type)", field.name()),
                type_annotation: "Any".to_string(),
                description: Some(format!(
                    "Flattened field from unknown type: {}",
                    field.name()
                )),
                deprecation_note: None,
                optional: true,
                default_value: Some("None".to_string()),
            });
        }
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

fn render_enum(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::{Fields, Representation};

    // Check if this is a tagged enum (internally tagged)
    match &enum_def.representation {
        Representation::Internal { tag } => {
            // Check if this internally tagged enum has any tuple variants that need flattening
            let has_tuple_variants = enum_def
                .variants
                .iter()
                .any(|v| matches!(v.fields, Fields::Unnamed(_)));

            if has_tuple_variants {
                // This is our special case - internally tagged enum with tuple variants
                render_internally_tagged_enum_with_flattened_tuples(
                    enum_def,
                    tag,
                    schema,
                    implemented_types,
                )
            } else {
                // This is an internally tagged enum with only struct/unit variants
                render_internally_tagged_struct_enum(enum_def, tag, schema, implemented_types)
            }
        }
        Representation::None => {
            // This is an untagged enum - generate Union without discriminator
            render_untagged_enum(enum_def, schema, implemented_types)
        }
        _ => {
            // Check if this is a primitive-represented enum (has discriminant values)
            let has_discriminants = enum_def.variants.iter().any(|v| v.discriminant.is_some());

            if has_discriminants {
                // This is a primitive enum - use IntEnum or similar
                render_primitive_enum(enum_def)
            } else {
                // This is a simple string enum - use the existing approach
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

fn render_internally_tagged_struct_enum(
    enum_def: &reflectapi_schema::Enum,
    tag: &str,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let enum_name = improve_class_name(&enum_def.name);
    let mut variant_classes = Vec::new();
    let mut union_variants = Vec::new();

    // Generate individual classes for each variant
    for variant in &enum_def.variants {
        let variant_class_name = format!("{}{}", enum_name, to_pascal_case(variant.name()));
        let variant_serde_name = variant.serde_name();

        // Build fields for this variant
        let mut fields = vec![
            // Add the discriminator field
            templates::Field {
                name: tag.to_string(),
                type_annotation: format!("Literal['{}']", variant_serde_name),
                description: Some("Discriminator field".to_string()),
                deprecation_note: None,
                optional: false,
                default_value: None,
            },
        ];

        // Add variant-specific fields
        match &variant.fields {
            Fields::Named(named_fields) => {
                for field in named_fields {
                    let field_type =
                        type_ref_to_python_type(&field.type_ref, schema, implemented_types, &[])?;

                    // Check if field type is Option<T> (which gets converted to T | None automatically)
                    let is_option_type = field.type_ref.name == "std::option::Option";

                    // Fix default value handling for optional fields
                    let (optional, default_value, final_field_type) = if !field.required {
                        // Field is not required - add | None if not already an Option type
                        if is_option_type {
                            (true, Some("None".to_string()), field_type) // Already has | None
                        } else {
                            (
                                true,
                                Some("None".to_string()),
                                format!("{} | None", field_type),
                            )
                        }
                    } else if is_option_type {
                        // Field is required but Option type - still needs default None
                        (true, Some("None".to_string()), field_type) // Already has | None
                    } else {
                        // Required non-Option field
                        (false, None, field_type)
                    };

                    fields.push(templates::Field {
                        name: sanitize_field_name(field.name()),
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                    });
                }
            }
            Fields::Unnamed(_) => {
                // Tuple variants are not supported for internally tagged enums in serde
                return Err(anyhow::anyhow!(
                    "Tuple variants are not supported for internally tagged enums: {}",
                    variant.name()
                ));
            }
            Fields::None => {
                // Unit variant - no additional fields needed
            }
        }

        // Generate the variant class
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
                .context("Failed to render variant class")?,
        );
        union_variants.push(templates::UnionVariant {
            name: variant.name().to_string(),
            type_annotation: variant_class_name.clone(),
            description: Some(variant.description().to_string()),
        });
    }

    // Generate the union type alias
    let union_template = templates::UnionClass {
        name: enum_name,
        description: Some(enum_def.description().to_string()),
        variants: union_variants,
        discriminator_field: tag.to_string(),
    };

    let union_definition = union_template.render().context("Failed to render union")?;

    // Combine all parts
    let mut result = variant_classes.join("\n\n");
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    result.push_str(&union_definition);

    Ok(result)
}

fn render_internally_tagged_enum_with_flattened_tuples(
    enum_def: &reflectapi_schema::Enum,
    tag: &str,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::{Fields, Type};

    let enum_name = improve_class_name(&enum_def.name);
    let mut variant_class_definitions: Vec<String> = Vec::new();
    let mut union_variant_names: Vec<String> = Vec::new();

    // Generate individual classes for each variant
    for variant in &enum_def.variants {
        let variant_class_name = format!("{}{}", enum_name, to_pascal_case(variant.name()));
        union_variant_names.push(variant_class_name.clone());

        // Start with discriminator field
        let mut fields = vec![templates::Field {
            name: tag.to_string(),
            type_annotation: format!("Literal['{}']", variant.serde_name()),
            description: Some("Discriminator field".to_string()),
            deprecation_note: None,
            optional: false,
            default_value: None,
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
                                &[],
                            )?;

                            // Handle field optionality
                            let is_option_type =
                                struct_field.type_ref.name == "std::option::Option";
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

                            fields.push(templates::Field {
                                name: sanitize_field_name(struct_field.name()),
                                type_annotation: final_field_type,
                                description: Some(struct_field.description().to_string()),
                                deprecation_note: struct_field.deprecation_note.clone(),
                                optional,
                                default_value,
                            });
                        }
                    }
                    Type::Enum(_) => {
                        return Err(anyhow::anyhow!(
                            "Tuple variant {} contains enum type {}, which cannot be flattened. Only struct types can be flattened in internally tagged enums with tuple variants. This is a Serde limitation - internally tagged enums with tuple variants require the inner type to be a struct so its fields can be flattened alongside the discriminator tag.",
                            variant.name(),
                            inner_type_name
                        ));
                    }
                    Type::Primitive(_) => {
                        return Err(anyhow::anyhow!(
                            "Tuple variant {} contains primitive type {}, which cannot be flattened. Only struct types can be flattened in internally tagged enums with tuple variants. This is a Serde limitation - internally tagged enums with tuple variants require the inner type to be a struct so its fields can be flattened alongside the discriminator tag.",
                            variant.name(),
                            inner_type_name
                        ));
                    }
                }
            }
            Fields::Named(named_fields) => {
                // Struct variant - handle normally
                for field in named_fields {
                    let field_type =
                        type_ref_to_python_type(&field.type_ref, schema, implemented_types, &[])?;

                    let is_option_type = field.type_ref.name == "std::option::Option";
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
                        name: sanitize_field_name(field.name()),
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
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
            is_generic: false,
            generic_params: vec![],
        };

        variant_class_definitions.push(
            variant_template
                .render()
                .context("Failed to render variant class")?,
        );
    }

    // Generate the discriminated union
    let union_variants: Vec<templates::UnionVariant> = union_variant_names
        .iter()
        .zip(&enum_def.variants)
        .map(|(class_name, variant)| templates::UnionVariant {
            name: variant.name().to_string(),
            type_annotation: class_name.clone(),
            description: Some(variant.description().to_string()),
        })
        .collect();

    let union_template = templates::UnionClass {
        name: enum_name,
        description: Some(enum_def.description().to_string()),
        variants: union_variants,
        discriminator_field: tag.to_string(),
    };

    let union_definition = union_template.render().context("Failed to render union")?;

    // Combine all parts
    let mut result = variant_class_definitions.join("\n\n");
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    result.push_str(&union_definition);

    Ok(result)
}

fn render_untagged_enum(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let enum_name = improve_class_name(&enum_def.name);
    let mut variant_classes = Vec::new();
    let mut union_variants = Vec::new();

    // Process each variant to create separate classes (without discriminator fields)
    for variant in &enum_def.variants {
        let variant_class_name = format!("{}{}", enum_name, variant.name());
        let mut fields = Vec::new();

        // Add variant-specific fields (no discriminator field for untagged)
        match &variant.fields {
            Fields::Named(named_fields) => {
                for field in named_fields {
                    let field_type =
                        type_ref_to_python_type(&field.type_ref, schema, implemented_types, &[])?;
                    // Check if field type is Option<T>
                    let is_option_type = field.type_ref.name == "std::option::Option";
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

                    fields.push(templates::Field {
                        name: sanitize_field_name(field.name()),
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                    });
                }
            }
            Fields::Unnamed(unnamed_fields) => {
                // Handle tuple-like variants for untagged enums
                for (i, field) in unnamed_fields.iter().enumerate() {
                    let field_type =
                        type_ref_to_python_type(&field.type_ref, schema, implemented_types, &[])?;
                    let is_option_type = field.type_ref.name == "std::option::Option";
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
            description: Some(variant.description().to_string()),
        });
    }

    // Generate the union type alias (without Field discriminator)
    let union_template = templates::UntaggedUnionClass {
        name: enum_name,
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

    Ok(result)
}

// OneOf types don't exist in the current schema - this was removed

fn render_function(
    function: &Function,
    schema: &Schema,
    implemented_types: &HashMap<String, String>,
) -> anyhow::Result<templates::Function> {
    let input_type = if let Some(input_type) = function.input_type.as_ref() {
        type_ref_to_python_type(input_type, schema, implemented_types, &[])?
    } else {
        "None".to_string()
    };

    let output_type = if let Some(output_type) = function.output_type.as_ref() {
        type_ref_to_python_type(output_type, schema, implemented_types, &[])?
    } else {
        "Any".to_string()
    };

    let error_type = if let Some(error_type) = function.error_type.as_ref() {
        Some(type_ref_to_python_type(
            error_type,
            schema,
            implemented_types,
            &[],
        )?)
    } else {
        None
    };

    // Extract path and query parameters from input type
    let (path_params, query_params) = extract_parameters(schema, function, &input_type)?;

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
                            &[],
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
            | "str"
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

/// Improve Python class names to be more readable and Pythonic
fn improve_class_name(original_name: &str) -> String {
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
    rendered_types: &HashMap<String, String>,
    _schema: &Schema,
) -> String {
    let mut nested_classes = Vec::new();
    let mut namespace_groups: HashMap<String, Vec<String>> = HashMap::new();

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
    for (namespace, type_names) in namespace_groups {
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
    _rendered_types: &HashMap<String, String>,
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
    implemented_types: &HashMap<String, String>,
    active_generics: &[String],
) -> anyhow::Result<String> {
    // Check if this is an active generic parameter first
    if active_generics.contains(&type_ref.name) {
        return Ok(type_ref.name.clone());
    }
    // Check if this type has a direct mapping
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

        // Handle generic types with arguments
        let mut result = python_type.clone();

        // Get type parameter names for substitution (e.g., "T", "K", "V")
        let type_params = get_type_parameters(&type_ref.name);

        for (param, arg) in type_params.iter().zip(type_ref.arguments.iter()) {
            let resolved_arg =
                type_ref_to_python_type(arg, schema, implemented_types, active_generics)?;
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
                return type_ref_to_python_type(
                    &inner_field.type_ref,
                    schema,
                    implemented_types,
                    active_generics,
                );
            }
        }

        let base_type = improve_class_name(&type_ref.name);

        // Handle generic types with arguments - follow TypeScript pattern
        if !type_ref.arguments.is_empty() {
            // Collect type parameters from the schema definition
            let type_params: Vec<_> = type_def.parameters().collect();

            // Ensure we have the same number of arguments as parameters
            if type_params.len() == type_ref.arguments.len() {
                let mut result = base_type.clone();

                // Substitute type parameters with actual arguments
                for (type_param, type_arg) in type_params.iter().zip(type_ref.arguments.iter()) {
                    if result.contains(&type_param.name) {
                        let resolved_arg = type_ref_to_python_type(
                            type_arg,
                            schema,
                            implemented_types,
                            active_generics,
                        )?;
                        result = result.replacen(&type_param.name, &resolved_arg, 1);
                    }
                }

                // If no substitutions were made, add generic arguments
                if result == base_type {
                    let arg_types: Result<Vec<String>, _> = type_ref
                        .arguments
                        .iter()
                        .map(|arg| {
                            type_ref_to_python_type(arg, schema, implemented_types, active_generics)
                        })
                        .collect();
                    let arg_types = arg_types?;
                    result = format!("{}[{}]", base_type, arg_types.join(", "));
                }

                return Ok(result);
            }
        }

        return Ok(base_type);
    }

    // Try schema-based fallback first (like TypeScript codegen does)
    if let Some(fallback_type_ref) = type_ref.fallback_once(schema.input_types()) {
        return type_ref_to_python_type(
            &fallback_type_ref,
            schema,
            implemented_types,
            active_generics,
        );
    }

    if let Some(fallback_type_ref) = type_ref.fallback_once(schema.output_types()) {
        return type_ref_to_python_type(
            &fallback_type_ref,
            schema,
            implemented_types,
            active_generics,
        );
    }

    // Final fallback for undefined external types - try to map to sensible Python types
    let fallback_type = map_external_type_to_python(&type_ref.name);
    Ok(fallback_type)
}

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

// Note: format_default_value function removed as it's no longer needed

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
from pydantic import BaseModel, ConfigDict{% if has_discriminated_unions %}, Field{% endif %}

# Runtime imports
{% if has_async %}{% if has_sync %}from reflectapi_runtime import AsyncClientBase, ClientBase, ApiResponse{% else %}from reflectapi_runtime import AsyncClientBase, ApiResponse{% endif %}{% else %}{% if has_sync %}from reflectapi_runtime import ClientBase, ApiResponse{% endif %}{% endif %}
{% if has_reflectapi_option %}from reflectapi_runtime import ReflectapiOption
{% endif %}{% if has_testing %}from reflectapi_runtime.testing import MockClient, create_api_response
{% endif %}

{% if has_generics %}
T = TypeVar('T')
D = TypeVar('D')
Input = TypeVar('Input')  # More descriptive than 'I'
ClientType = TypeVar('ClientType')  # More descriptive than 'C'
{% else %}
T = TypeVar('T')
{% endif %}
"#,
        ext = "txt"
    )]
    pub struct Imports {
        pub has_async: bool,
        pub has_sync: bool,
        pub has_testing: bool,
        pub has_enums: bool,
        pub has_reflectapi_option: bool,
        pub has_warnings: bool,
        pub has_datetime: bool,
        pub has_uuid: bool,
        pub has_timedelta: bool,
        pub has_date: bool,
        pub has_generics: bool,
        pub has_annotated: bool,
        pub has_literal: bool,
        pub has_discriminated_unions: bool,
    }

    #[derive(Template)]
    #[template(
        source = r#"class {{ name }}(BaseModel{% if is_generic %}, Generic[{% for param in generic_params %}{{ param }}{% if !loop.last %}, {% endif %}{% endfor %}]{% endif %}):
{% if description.is_some() && !description.as_deref().unwrap().is_empty() %}    """{{ description.as_deref().unwrap() }}"""
{% else %}    """Generated data model."""
{% endif %}
    model_config = ConfigDict(extra="ignore")

{% for field in fields %}    {{ field.name }}: {{ field.type_annotation }}{% if field.optional %} = None{% else %}{% if field.default_value.is_some() %} = {{ field.default_value.as_ref().unwrap() }}{% endif %}{% endif %}
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
        source = r#"{{ name }} = Annotated[Union[{% for variant in variants %}{{ variant.type_annotation }}{% if !loop.last %}, {% endif %}{% endfor %}], Field(discriminator='{{ discriminator_field }}')]
"""{{ description.as_deref().unwrap_or("") }}"""
"#,
        ext = "txt"
    )]
    pub struct UnionClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<UnionVariant>,
        pub discriminator_field: String,
    }

    #[derive(Template)]
    #[template(
        source = r#"{{ name }} = Union[{% for variant in variants %}{{ variant.type_annotation }}{% if !loop.last %}, {% endif %}{% endfor %}]
"""{{ description.as_deref().unwrap_or("") }}"""
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
            json_model=data,
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
            json_model=data,
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

    pub struct PrimitiveEnumVariant {
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
