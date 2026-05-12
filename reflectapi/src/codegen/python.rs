use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::LazyLock;

use crate::codegen::schema as schema_codegen;
use crate::{Schema, TypeReference};
use reflectapi_schema::{Function, OutputType, Type};

/// Sanitize text for inclusion in a Python triple-quoted docstring.
/// Escapes backslashes (which act as line continuation) and triple-quote
/// sequences (which would close the docstring prematurely).
fn sanitize_for_docstring(text: &str) -> String {
    text.replace('\\', "\\\\").replace("\"\"\"", "\\\"\\\"\\\"")
}

/// Sanitize text for inclusion in a Python double-quoted string literal.
/// Escapes backslashes, double quotes, and replaces newlines with \n.
fn sanitize_for_string_literal(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
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
    /// Attempt to format the generated code with ruff. Will fall back to basic
    /// formatting if ruff is not available.
    pub format: bool,
    /// Base URL for the API (optional)
    pub base_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            package_name: "api_client".to_string(),
            generate_async: true,
            generate_sync: true,
            generate_testing: false,
            format: true,
            base_url: None,
        }
    }
}

#[derive(Default)]
struct PythonMetadataUsage {
    stdlib_imports: BTreeSet<String>,
    runtime_imports: BTreeSet<String>,
    runtime_provided_types: BTreeSet<String>,
}

/// Resolve Python field optionality: determines whether a field needs a default
/// value of `None` and whether `| None` should be appended to its type hint.
fn resolve_field_optionality(
    type_name: &str,
    field_type: String,
    required: bool,
) -> (bool, Option<String>, String) {
    let is_option_type = type_name == "std::option::Option" || type_name == "reflectapi::Option";
    if !required {
        if is_option_type {
            (true, Some("None".to_string()), field_type)
        } else {
            (
                true,
                Some("None".to_string()),
                format!("{field_type} | None"),
            )
        }
    } else if is_option_type {
        (true, Some("None".to_string()), field_type)
    } else {
        (false, None, field_type)
    }
}

struct PythonTypeMapping {
    type_hint: &'static str,
    imports: &'static [&'static str],
    runtime_imports: &'static [&'static str],
    provided_by_runtime: bool,
    ignore_type_arguments: bool,
}

impl PythonTypeMapping {
    const fn simple(type_hint: &'static str) -> Self {
        Self {
            type_hint,
            imports: &[],
            runtime_imports: &[],
            provided_by_runtime: false,
            ignore_type_arguments: false,
        }
    }
}

fn python_type_mappings() -> &'static HashMap<&'static str, PythonTypeMapping> {
    static MAPPINGS: LazyLock<HashMap<&'static str, PythonTypeMapping>> = LazyLock::new(|| {
        let mut m = HashMap::new();

        // Integer types
        for name in [
            "i8",
            "i16",
            "i32",
            "i64",
            "u8",
            "u16",
            "u32",
            "u64",
            "isize",
            "usize",
            "std::num::NonZeroU8",
            "std::num::NonZeroU16",
            "std::num::NonZeroU32",
            "std::num::NonZeroU64",
            "std::num::NonZeroU128",
        ] {
            m.insert(name, PythonTypeMapping::simple("int"));
        }

        // Float types
        m.insert("f32", PythonTypeMapping::simple("float"));
        m.insert("f64", PythonTypeMapping::simple("float"));

        // Bool
        m.insert("bool", PythonTypeMapping::simple("bool"));

        // String types
        for name in [
            "String",
            "std::string::String",
            "url::Url",
            "rust_decimal::Decimal",
            "chrono_tz::Tz",
        ] {
            m.insert(name, PythonTypeMapping::simple("str"));
        }

        // Collections
        m.insert("std::vec::Vec", PythonTypeMapping::simple("list[T]"));
        for name in [
            "std::collections::HashMap",
            "std::collections::BTreeMap",
            "indexmap::IndexMap",
        ] {
            m.insert(name, PythonTypeMapping::simple("dict[K, V]"));
        }

        // Option / Result
        m.insert("std::option::Option", PythonTypeMapping::simple("T | None"));
        m.insert("std::result::Result", PythonTypeMapping::simple("T | E"));

        // ReflectAPI runtime types
        // The three-state Option from reflectapi-schema renders as plain
        // `T | None`. The "absent on the wire" state is preserved at the
        // model level: classes with any `reflectapi::Option<_>` field
        // inherit from `ReflectapiPartialModel`, whose `@model_serializer`
        // drops fields not in `model_fields_set`.
        m.insert("reflectapi::Option", PythonTypeMapping::simple("T | None"));
        m.insert(
            "reflectapi::Empty",
            PythonTypeMapping {
                type_hint: "ReflectapiEmpty",
                imports: &[],
                runtime_imports: &["ReflectapiEmpty"],
                provided_by_runtime: true,
                ignore_type_arguments: false,
            },
        );
        m.insert(
            "reflectapi::Infallible",
            PythonTypeMapping {
                type_hint: "ReflectapiInfallible",
                imports: &[],
                runtime_imports: &["ReflectapiInfallible"],
                provided_by_runtime: true,
                ignore_type_arguments: false,
            },
        );

        // Date/time types
        for name in ["chrono::DateTime", "chrono::NaiveDateTime"] {
            m.insert(
                name,
                PythonTypeMapping {
                    type_hint: "datetime",
                    imports: &["from datetime import datetime"],
                    runtime_imports: &[],
                    provided_by_runtime: false,
                    ignore_type_arguments: true,
                },
            );
        }
        m.insert(
            "chrono::NaiveDate",
            PythonTypeMapping {
                type_hint: "date",
                imports: &["from datetime import date"],
                runtime_imports: &[],
                provided_by_runtime: false,
                ignore_type_arguments: false,
            },
        );
        m.insert(
            "uuid::Uuid",
            PythonTypeMapping {
                type_hint: "UUID",
                imports: &["from uuid import UUID"],
                runtime_imports: &[],
                provided_by_runtime: false,
                ignore_type_arguments: false,
            },
        );
        m.insert(
            "std::time::Duration",
            PythonTypeMapping {
                // serde renders Duration as `{"secs": <u64>, "nanos": <u32>}`.
                // Pydantic's bare `timedelta` validator rejects that
                // dict shape (it expects ISO-8601 / int / float).
                // `ReflectapiDuration` is `Annotated[timedelta,
                // BeforeValidator(...), PlainSerializer(...)]` —
                // accepts both the serde wire shape and python
                // timedelta values, emits the serde shape on the way
                // out.
                type_hint: "ReflectapiDuration",
                imports: &[],
                runtime_imports: &["ReflectapiDuration"],
                provided_by_runtime: true,
                ignore_type_arguments: false,
            },
        );

        // Special types
        m.insert("std::tuple::Tuple0", PythonTypeMapping::simple("None"));
        // Rust tuples of arity >=1 serialise as JSON arrays of the same
        // arity. Pydantic's `tuple[A, B, …]` annotation validates that
        // exactly: a fixed-length heterogeneous sequence. Anything not
        // mapped here would otherwise render via
        // `type_name_to_python_ref` as `std.tuple.TupleN[A, B]`, which
        // is a dangling reference (there's no `std.tuple` submodule).
        // Cover up to Tuple16 — matches the arity ceiling enforced by
        // `reflectapi-schema` and Rust's own std trait impls.
        for arity in 1..=16 {
            let name: &'static str = match arity {
                1 => "std::tuple::Tuple1",
                2 => "std::tuple::Tuple2",
                3 => "std::tuple::Tuple3",
                4 => "std::tuple::Tuple4",
                5 => "std::tuple::Tuple5",
                6 => "std::tuple::Tuple6",
                7 => "std::tuple::Tuple7",
                8 => "std::tuple::Tuple8",
                9 => "std::tuple::Tuple9",
                10 => "std::tuple::Tuple10",
                11 => "std::tuple::Tuple11",
                12 => "std::tuple::Tuple12",
                13 => "std::tuple::Tuple13",
                14 => "std::tuple::Tuple14",
                15 => "std::tuple::Tuple15",
                16 => "std::tuple::Tuple16",
                _ => unreachable!(),
            };
            m.insert(name, PythonTypeMapping::simple("tuple"));
        }
        m.insert("serde_json::Value", PythonTypeMapping::simple("Any"));

        // Wrapper types (transparent)
        for name in ["std::boxed::Box", "std::sync::Arc", "std::rc::Rc"] {
            m.insert(name, PythonTypeMapping::simple("T"));
        }

        // Path types
        for name in ["std::path::PathBuf", "std::path::Path"] {
            m.insert(
                name,
                PythonTypeMapping {
                    type_hint: "Path",
                    imports: &["from pathlib import Path"],
                    runtime_imports: &[],
                    provided_by_runtime: false,
                    ignore_type_arguments: false,
                },
            );
        }

        // Network types
        m.insert(
            "std::net::IpAddr",
            PythonTypeMapping {
                type_hint: "IPv4Address | IPv6Address",
                imports: &["from ipaddress import IPv4Address, IPv6Address"],
                runtime_imports: &[],
                provided_by_runtime: false,
                ignore_type_arguments: false,
            },
        );
        m.insert(
            "std::net::Ipv4Addr",
            PythonTypeMapping {
                type_hint: "IPv4Address",
                imports: &["from ipaddress import IPv4Address"],
                runtime_imports: &[],
                provided_by_runtime: false,
                ignore_type_arguments: false,
            },
        );
        m.insert(
            "std::net::Ipv6Addr",
            PythonTypeMapping {
                type_hint: "IPv6Address",
                imports: &["from ipaddress import IPv6Address"],
                runtime_imports: &[],
                provided_by_runtime: false,
                ignore_type_arguments: false,
            },
        );

        m
    });
    &MAPPINGS
}

fn collect_python_metadata_usage(all_type_names: &[String]) -> PythonMetadataUsage {
    let mut usage = PythonMetadataUsage::default();
    let mappings = python_type_mappings();

    for type_name in all_type_names {
        let Some(mapping) = mappings.get(type_name.as_str()) else {
            continue;
        };

        for import in mapping.imports {
            usage.stdlib_imports.insert((*import).to_string());
        }
        for import in mapping.runtime_imports {
            usage.runtime_imports.insert((*import).to_string());
        }
        if mapping.provided_by_runtime {
            usage.runtime_provided_types.insert(type_name.clone());
        }
    }

    usage
}

/// Generate optimized imports with proper sorting and deduplication
fn generate_optimized_imports(imports: &templates::Imports) -> String {
    let mut stdlib_imports = BTreeSet::new();
    let mut typing_imports = BTreeSet::new();
    let mut third_party_imports = BTreeSet::new();
    let mut runtime_imports: BTreeSet<String> = BTreeSet::new();

    // Standard library - enum
    if imports.has_enums {
        stdlib_imports.insert("from enum import Enum".to_string());
    }

    // Standard library - warnings
    if imports.has_warnings {
        stdlib_imports.insert("import warnings".to_string());
    }

    stdlib_imports.extend(imports.extra_stdlib_imports.iter().cloned());

    // Streaming methods need Iterator / AsyncIterator from collections.abc.
    if imports.has_streaming {
        let mut names = Vec::new();
        if imports.has_sync {
            names.push("Iterator");
        }
        if imports.has_async {
            names.push("AsyncIterator");
        }
        if !names.is_empty() {
            stdlib_imports.insert(format!("from collections.abc import {}", names.join(", ")));
        }
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

    // Pydantic imports — Field is always imported since it's used for
    // descriptions, aliases, discriminators, and defaults across many contexts.
    third_party_imports.insert("BaseModel");
    third_party_imports.insert("ConfigDict");
    third_party_imports.insert("Field");
    if imports.has_discriminated_unions {
        third_party_imports.insert("RootModel");
    }
    if imports.has_externally_tagged_enums {
        third_party_imports.insert("RootModel");
        third_party_imports.insert("model_validator");
        third_party_imports.insert("model_serializer");
        runtime_imports.insert(
            "parse_externally_tagged as _parse_externally_tagged, serialize_externally_tagged as _serialize_externally_tagged".to_string(),
        );
    }

    // Runtime imports - client bases
    if imports.has_async && imports.has_sync {
        runtime_imports.insert("AsyncClientBase, ClientBase, ApiResponse".to_string());
    } else if imports.has_async {
        runtime_imports.insert("AsyncClientBase, ApiResponse".to_string());
    } else if imports.has_sync {
        runtime_imports.insert("ClientBase, ApiResponse".to_string());
    }

    if imports.has_partial_models {
        runtime_imports.insert("ReflectapiPartialModel".to_string());
    }

    runtime_imports.extend(imports.extra_runtime_imports.iter().cloned());

    // Build the final import string
    let mut result = Vec::new();

    // Add header
    result.push("# Standard library imports".to_string());
    let mut merged_from_imports: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut plain_stdlib_imports = Vec::new();
    for import in stdlib_imports {
        if let Some(rest) = import.strip_prefix("from ") {
            if let Some((module, names)) = rest.split_once(" import ") {
                let entry = merged_from_imports.entry(module.to_string()).or_default();
                for name in names.split(", ") {
                    entry.insert(name.to_string());
                }
                continue;
            }
        }
        plain_stdlib_imports.push(import);
    }
    for import in plain_stdlib_imports {
        result.push(import);
    }
    for (module, names) in merged_from_imports {
        let mut names = names.into_iter().collect::<Vec<_>>();
        if module == "datetime" {
            let rank = |name: &str| match name {
                "datetime" => 0,
                "date" => 1,
                "timedelta" => 2,
                _ => 99,
            };
            names.sort_by_key(|name| (rank(name), name.clone()));
        }
        result.push(format!("from {module} import {}", names.join(", ")));
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
        result.push(format!("from reflectapi_runtime import {import}"));
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
/// Collect all TypeVars used by a type.
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

/// Render a struct that contains flattened fields using direct field expansion.
///
/// For flattened structs, fields are expanded inline into the parent model.
/// For flattened internally-tagged enums, we generate per-variant models that
/// merge the parent's fields with each variant's fields + the tag discriminator,
/// then emit a discriminated union RootModel. This matches the wire format that
/// serde produces with `#[serde(flatten)]` on internally-tagged enums.
fn render_struct_with_flatten(
    struct_def: &reflectapi_schema::Struct,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    let struct_name = python_class_name(&struct_def.name, class_names);
    let active_generics: Vec<String> = struct_def
        .parameters
        .iter()
        .map(|p| p.name.clone())
        .collect();

    for generic in &active_generics {
        used_type_vars.insert(generic.clone());
    }

    // Check if any flattened field is an internally-tagged enum
    let flattened_internal_enum =
        struct_def
            .fields
            .iter()
            .filter(|f| f.flattened())
            .find_map(|field| {
                let type_name = resolve_flattened_type_name(&field.type_ref);
                match schema.get_type(type_name) {
                    Some(reflectapi_schema::Type::Enum(enum_def)) => {
                        match &enum_def.representation {
                            reflectapi_schema::Representation::Internal { tag } => {
                                Some((field, enum_def.clone(), tag.clone()))
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                }
            });

    if let Some((_enum_field, enum_def, tag)) = flattened_internal_enum {
        // Wire-compatible path: generate per-variant models with merged fields
        render_struct_with_flattened_internal_enum(
            struct_def,
            &struct_name,
            &enum_def,
            &tag,
            schema,
            implemented_types,
            &active_generics,
            used_type_vars,
        )
    } else {
        // Standard path: expand all flattened fields inline
        render_struct_with_flatten_standard(
            struct_def,
            &struct_name,
            schema,
            implemented_types,
            &active_generics,
            used_type_vars,
        )
    }
}

/// Resolve the target type name for a flattened field, unwrapping `Option<T>`
fn resolve_flattened_type_name(type_ref: &TypeReference) -> &str {
    if (type_ref.name == "std::option::Option" || type_ref.name == "reflectapi::Option")
        && !type_ref.arguments.is_empty()
    {
        &type_ref.arguments[0].name
    } else {
        &type_ref.name
    }
}

/// Render a struct with a flattened internally-tagged enum by generating
/// per-variant models that merge parent fields + variant fields + tag.
#[allow(clippy::too_many_arguments)]
fn render_struct_with_flattened_internal_enum(
    struct_def: &reflectapi_schema::Struct,
    struct_name: &str,
    enum_def: &reflectapi_schema::Enum,
    tag: &str,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let mut output = String::new();
    let mut union_variant_names: Vec<String> = Vec::new();

    // Collect the parent struct's non-flattened fields + any flattened struct fields
    let mut base_fields: Vec<templates::Field> = Vec::new();
    for field in struct_def.fields.iter().filter(|f| !f.flattened()) {
        let (python_name, alias) = sanitize_field_name_with_alias(field.name(), field.serde_name());
        let field_type = type_ref_to_python_type(
            &field.type_ref,
            schema,
            implemented_types,
            active_generics,
            used_type_vars,
        )?;
        base_fields.push(templates::Field {
            name: python_name,
            type_annotation: if field.required {
                field_type
            } else {
                format!("{field_type} | None")
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

            is_partial: field.type_ref.name == "reflectapi::Option",
        });
    }

    // Also expand any flattened struct fields (non-enum) into base_fields
    for field in struct_def.fields.iter().filter(|f| f.flattened()) {
        let type_name = resolve_flattened_type_name(&field.type_ref);
        if let Some(reflectapi_schema::Type::Struct(_)) = schema.get_type(type_name) {
            let flattened = collect_flattened_fields(
                &field.type_ref,
                schema,
                implemented_types,
                active_generics,
                field.required,
                0,
                used_type_vars,
                Some(field.name()),
            )?;
            base_fields.extend(flattened);
        // Enum fields are handled below as variants
        } else if let Some(reflectapi_schema::Type::Enum(enum_def)) = schema.get_type(type_name) {
            // Check if this is the internally-tagged enum we're expanding
            // (it's handled below as variants). Other enums get emitted as regular fields.
            let is_the_expanding_enum = matches!(
                &enum_def.representation,
                reflectapi_schema::Representation::Internal { .. }
            );
            if !is_the_expanding_enum {
                let field_type = type_ref_to_python_type(
                    &field.type_ref,
                    schema,
                    implemented_types,
                    active_generics,
                    used_type_vars,
                )?;
                let (python_name, alias) =
                    sanitize_field_name_with_alias(field.name(), field.serde_name());
                base_fields.push(templates::Field {
                    name: python_name,
                    type_annotation: if field.required {
                        field_type
                    } else {
                        format!("{field_type} | None")
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

                    is_partial: field.type_ref.name == "reflectapi::Option",
                });
            }
        }
    }

    // Generate per-variant models
    for variant in &enum_def.variants {
        let variant_class_name = format!("{}{}", struct_name, to_pascal_case(variant.name()));
        union_variant_names.push(variant_class_name.clone());

        // Start with base fields from the parent struct
        let mut fields = base_fields.clone();

        // Add the tag discriminator field
        let (sanitized_tag, tag_alias) = sanitize_field_name_with_alias(tag, tag);
        fields.push(templates::Field {
            name: sanitized_tag.clone(),
            type_annotation: format!("Literal['{}']", variant.serde_name()),
            description: Some("Discriminator field".to_string()),
            deprecation_note: None,
            optional: false,
            default_value: Some(format!("\"{}\"", variant.serde_name())),
            alias: tag_alias.clone(),

            is_partial: false,
        });

        // Add variant-specific fields
        match &variant.fields {
            Fields::Named(named_fields) => {
                for vf in named_fields {
                    let field_type = type_ref_to_python_type(
                        &vf.type_ref,
                        schema,
                        implemented_types,
                        active_generics,
                        used_type_vars,
                    )?;
                    let (optional, default_value, final_type) =
                        resolve_field_optionality(&vf.type_ref.name, field_type, vf.required);
                    let (sanitized, alias) =
                        sanitize_field_name_with_alias(vf.name(), vf.serde_name());
                    fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_type,
                        description: Some(vf.description().to_string()),
                        deprecation_note: vf.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,

                        is_partial: vf.type_ref.name == "reflectapi::Option",
                    });
                }
            }
            Fields::Unnamed(unnamed_fields) if unnamed_fields.len() == 1 => {
                // Tuple variant with one field — expand inner struct fields
                let inner = &unnamed_fields[0];
                let inner_name = if inner.type_ref.name == "std::boxed::Box" {
                    inner
                        .type_ref
                        .arguments
                        .first()
                        .map(|a| a.name.as_str())
                        .unwrap_or(&inner.type_ref.name)
                } else {
                    &inner.type_ref.name
                };
                if let Some(reflectapi_schema::Type::Struct(inner_struct)) =
                    schema.get_type(inner_name)
                {
                    for sf in inner_struct.fields.iter() {
                        if sf.flattened() {
                            // The inner struct has its own flatten — expand
                            // those fields inline so the wire shape is
                            // preserved across the tuple-variant boundary.
                            let nested = collect_flattened_fields(
                                &sf.type_ref,
                                schema,
                                implemented_types,
                                active_generics,
                                sf.required,
                                0,
                                used_type_vars,
                                Some(sf.name()),
                            )?;
                            fields.extend(nested);
                            continue;
                        }
                        let field_type = type_ref_to_python_type(
                            &sf.type_ref,
                            schema,
                            implemented_types,
                            active_generics,
                            used_type_vars,
                        )?;
                        let (sanitized, alias) =
                            sanitize_field_name_with_alias(sf.name(), sf.serde_name());
                        fields.push(templates::Field {
                            name: sanitized,
                            type_annotation: if sf.required {
                                field_type
                            } else {
                                format!("{field_type} | None")
                            },
                            description: Some(sf.description().to_string()),
                            deprecation_note: sf.deprecation_note.clone(),
                            optional: !sf.required,
                            default_value: if sf.required {
                                None
                            } else {
                                Some("None".to_string())
                            },
                            alias,

                            is_partial: sf.type_ref.name == "reflectapi::Option",
                        });
                    }
                }
            }
            Fields::None => {
                // Unit variant — no additional fields beyond tag
            }
            _ => {
                // Multi-field tuple variant — not supported for flattening
            }
        }

        // Render the variant class
        let variant_template = templates::DataClass {
            name: variant_class_name,
            description: Some(format!("'{}' variant of {}", variant.name(), struct_name)),
            fields,
            is_tuple: false,
            is_generic: !active_generics.is_empty(),
            generic_params: active_generics.to_vec(),
        };
        output.push_str(&variant_template.render());
        output.push('\n');
    }

    // Handle empty enum (no variants)
    if union_variant_names.is_empty() {
        output.push_str(&format!(
            "\nclass {struct_name}(RootModel):\n    \"\"\"Empty discriminated union (no variants)\"\"\"\n    root: None = None\n"
        ));
        return Ok(output);
    }

    // Render the parent type as a discriminated union RootModel
    let (sanitized_tag, _) = sanitize_field_name_with_alias(tag, tag);
    let union_type = union_variant_names.join(",\n            ");
    output.push_str(&format!(
        "\nclass {struct_name}(RootModel):\n    root: Annotated[\n        Union[\n            {union_type},\n        ],\n        Field(discriminator=\"{sanitized_tag}\"),\n    ]\n"
    ));

    // Note: model_rebuild() for per-variant classes is handled by the global
    // rebuild section after namespace classes are defined. Inline rebuild here
    // would fail because dotted namespace references (used in type annotations)
    // cannot be resolved until namespace classes exist.

    Ok(output)
}

/// Standard flatten path: expand all flattened struct fields inline into a single model.
fn render_struct_with_flatten_standard(
    struct_def: &reflectapi_schema::Struct,
    struct_name: &str,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    let mut all_fields = Vec::new();

    // Add regular fields
    for field in struct_def.fields.iter().filter(|f| !f.flattened()) {
        let (python_name, alias) = sanitize_field_name_with_alias(field.name(), field.serde_name());
        let field_type = type_ref_to_python_type(
            &field.type_ref,
            schema,
            implemented_types,
            active_generics,
            used_type_vars,
        )?;

        all_fields.push(templates::Field {
            name: python_name,
            type_annotation: if field.required {
                field_type
            } else {
                format!("{field_type} | None")
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

            is_partial: field.type_ref.name == "reflectapi::Option",
        });
    }

    // Add flattened fields (expanded directly)
    for field in struct_def.fields.iter().filter(|f| f.flattened()) {
        let flattened_fields = collect_flattened_fields(
            &field.type_ref,
            schema,
            implemented_types,
            active_generics,
            field.required,
            0,
            used_type_vars,
            Some(field.name()),
        )?;
        all_fields.extend(flattened_fields);
    }

    let struct_template = templates::DataClass {
        name: struct_name.to_string(),
        description: Some(struct_def.description().to_string()),
        fields: all_fields,
        is_tuple: false,
        is_generic: !active_generics.is_empty(),
        generic_params: active_generics.to_vec(),
    };

    let rendered = struct_template.render();
    Ok(rendered)
}

/// Validate that all type references in the schema exist
/// Drop every `PhantomData<T>` field from every struct/variant in
/// both input and output typespaces. PhantomData is purely a Rust
/// type-system marker — serde renders it as `null` and the field
/// carries no information for a wire consumer. Leaving it in the
/// schema forced the Python codegen to render
/// `std.marker.PhantomData[T]` annotations that didn't resolve.
fn strip_phantom_data_fields(schema: &mut Schema) {
    fn strip_from_typespace(
        typespace_name_iter: impl IntoIterator<Item = String>,
        ts: &mut reflectapi_schema::Typespace,
    ) {
        for name in typespace_name_iter {
            let Some(ty) = ts.get_type_mut(&name) else {
                continue;
            };
            match ty {
                reflectapi_schema::Type::Struct(s) => {
                    s.fields
                        .retain(|f| f.type_ref.name != "std::marker::PhantomData");
                }
                reflectapi_schema::Type::Enum(e) => {
                    for v in &mut e.variants {
                        v.fields
                            .retain(|f| f.type_ref.name != "std::marker::PhantomData");
                    }
                }
                _ => {}
            }
        }
    }
    let input_names: Vec<String> = schema
        .input_types
        .types()
        .map(|t| t.name().to_string())
        .collect();
    strip_from_typespace(input_names, &mut schema.input_types);
    let output_names: Vec<String> = schema
        .output_types
        .types()
        .map(|t| t.name().to_string())
        .collect();
    strip_from_typespace(output_names, &mut schema.output_types);
}

/// Walk every input/output struct in the schema and return `true` if
/// any field uses the three-state `reflectapi::Option<T>`. Used to
/// decide whether to import `ReflectapiPartialModel` into the generated
/// bundle. Cheap — runs once per generation.
fn schema_has_partial_field(schema: &Schema) -> bool {
    fn iter_typespace_has_partial(typespace: &reflectapi_schema::Typespace) -> bool {
        typespace.types().any(|t| match t {
            reflectapi_schema::Type::Struct(s) => {
                s.fields().any(|f| f.type_ref.name == "reflectapi::Option")
            }
            reflectapi_schema::Type::Enum(e) => e
                .variants
                .iter()
                .any(|v| v.fields().any(|f| f.type_ref.name == "reflectapi::Option")),
            _ => false,
        })
    }
    iter_typespace_has_partial(&schema.input_types)
        || iter_typespace_has_partial(&schema.output_types)
}

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
    let generation = build_python_generation(schema, config)?;
    render_package_files(generation, config)
}

#[cfg(test)]
fn generate_init_py(config: &Config) -> String {
    let mut imports = vec!["AsyncClient"];

    if config.generate_sync {
        imports.push("Client");
    }

    let imports_list = format!("{imports:?}");
    format!(
        r#"{}
    from ._client import {}

    __all__ = {}
    "#,
        templates::header_docstring(&config.package_name, ""),
        imports.join(", "),
        imports_list
    )
}

struct PythonGeneration {
    preamble: String,
    schema_name: String,
    schema_description: String,
    module_tree: templates::Module,
    rebuild_models: Vec<String>,
    client_code: String,
    testing_code: Option<String>,
}

fn collect_type_refs(type_ref: &TypeReference, type_refs: &mut Vec<TypeReference>) {
    type_refs.push(type_ref.clone());
    for argument in &type_ref.arguments {
        collect_type_refs(argument, type_refs);
    }
}

fn collect_concrete_type_refs(schema: &Schema) -> Vec<TypeReference> {
    let mut type_refs = Vec::new();

    for function in schema.functions() {
        if let Some(input_type) = &function.input_type {
            collect_type_refs(input_type, &mut type_refs);
        }
        if let Some(input_headers) = &function.input_headers {
            collect_type_refs(input_headers, &mut type_refs);
        }
        match &function.output_type {
            OutputType::Complete {
                output_type: Some(output_type),
            } => collect_type_refs(output_type, &mut type_refs),
            OutputType::Stream { item_type } => collect_type_refs(item_type, &mut type_refs),
            OutputType::Complete { output_type: None } => {}
        }
        if let Some(error_type) = &function.error_type {
            collect_type_refs(error_type, &mut type_refs);
        }
    }

    type_refs
}

fn type_ref_pascal_suffix(type_ref: &TypeReference) -> String {
    let mut suffix = python_class_name(&type_ref.name, &BTreeMap::new());
    for argument in &type_ref.arguments {
        suffix.push_str(&type_ref_pascal_suffix(argument));
    }
    suffix
}

fn concrete_type_ref_name(type_ref: &TypeReference) -> String {
    format!(
        "{}{}",
        python_class_name(&type_ref.name, &BTreeMap::new()),
        type_ref
            .arguments
            .iter()
            .map(type_ref_pascal_suffix)
            .collect::<Vec<_>>()
            .join("")
    )
}

fn substitute_type_params(
    type_ref: &TypeReference,
    substitutions: &BTreeMap<String, TypeReference>,
) -> TypeReference {
    if type_ref.arguments.is_empty() {
        if let Some(substitution) = substitutions.get(&type_ref.name) {
            return substitution.clone();
        }
    }

    TypeReference {
        name: type_ref.name.clone(),
        arguments: type_ref
            .arguments
            .iter()
            .map(|argument| substitute_type_params(argument, substitutions))
            .collect(),
    }
}

fn concrete_struct_from_type_ref(
    type_ref: &TypeReference,
    struct_def: &reflectapi_schema::Struct,
) -> reflectapi_schema::Struct {
    let substitutions: BTreeMap<String, TypeReference> = struct_def
        .parameters
        .iter()
        .zip(type_ref.arguments.iter())
        .map(|(parameter, argument)| (parameter.name.clone(), argument.clone()))
        .collect();

    let mut concrete = struct_def.clone();
    concrete.name = concrete_type_ref_name(type_ref);
    concrete.parameters.clear();

    match &mut concrete.fields {
        reflectapi_schema::Fields::Named(fields) | reflectapi_schema::Fields::Unnamed(fields) => {
            for field in fields {
                field.type_ref = substitute_type_params(&field.type_ref, &substitutions);
            }
        }
        reflectapi_schema::Fields::None => {}
    }

    concrete
}

fn render_concrete_flattened_generic_structs(
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
) -> anyhow::Result<BTreeMap<String, String>> {
    let mut rendered_types = BTreeMap::new();
    let mut used_type_vars = BTreeSet::new();

    for type_ref in collect_concrete_type_refs(schema) {
        if type_ref.arguments.is_empty()
            || rendered_types.contains_key(&concrete_type_ref_name(&type_ref))
        {
            continue;
        }

        let Some(Type::Struct(struct_def)) = schema.get_type(&type_ref.name) else {
            continue;
        };
        if struct_def.parameters.is_empty() || !struct_def.fields().any(|field| field.flattened()) {
            continue;
        }

        let concrete = concrete_struct_from_type_ref(&type_ref, struct_def);
        let rendered = render_struct(
            &concrete,
            schema,
            implemented_types,
            class_names,
            &mut used_type_vars,
        )?;
        rendered_types.insert(concrete.name.clone(), rendered);
    }

    Ok(rendered_types)
}

/// Build a tree of namespace modules from rendered type strings.
///
/// Each original type name is split on `::`.  The last segment is the leaf
/// type (already rendered); the preceding segments define the namespace path.
/// The rendered code for each type is placed into the leaf module.
fn modules_from_rendered_types(
    original_type_names: Vec<String>,
    mut rendered_types: BTreeMap<String, String>,
) -> templates::Module {
    use indexmap::IndexMap;

    let mut root_module = templates::Module {
        name: String::new(),
        types: vec![],
        submodules: IndexMap::new(),
    };

    for original_type_name in original_type_names {
        let mut module = &mut root_module;
        let mut parts: Vec<&str> = original_type_name.split("::").collect();
        parts.pop(); // Remove the leaf type name — already embedded in the rendered code.
        for part in parts {
            module = module
                .submodules
                .entry(part.to_string())
                .or_insert_with(|| templates::Module {
                    name: part.to_string(),
                    types: vec![],
                    submodules: IndexMap::new(),
                });
        }
        if let Some(rendered_type) = rendered_types.remove(&original_type_name) {
            module.types.push(templates::ModuleType {
                rendered: rendered_type,
                rust_path: original_type_name.clone(),
            });
        }
    }

    root_module
}

fn build_python_generation(
    mut schema: Schema,
    config: &Config,
) -> anyhow::Result<PythonGeneration> {
    // PhantomData<T> carries no wire data in Rust. serde serialises
    // each occurrence as `null`, but the field still appears in the
    // reflectapi schema with `type_ref.name == "std::marker::PhantomData"`.
    // Render-time the codegen used to emit
    // `std.marker.PhantomData[T]`, a dangling reference that broke
    // `model_rebuild()`. Strip the fields entirely — they have no
    // observable behaviour for a Python client.
    strip_phantom_data_fields(&mut schema);

    // Consolidate input/output types FIRST so both the SemanticSchema and
    // the raw Schema share the same unified type names.
    let all_type_names = schema.consolidate_types();
    // Pydantic cannot represent `serde(flatten)` over a generic parameter
    // at runtime — the inner T's wire-level fields would silently be
    // dropped. Monomorphize: emit one concrete class per (struct, args)
    // pair so the flatten resolves against a concrete type.
    monomorphize_flatten_generics(&mut schema)?;
    let implemented_types = build_implemented_types();
    validate_type_references(&schema)?;

    // Build the semantic IR with a codegen-specific pipeline that skips
    // TypeConsolidation (already done above) and NamingResolution (which
    // would rename types and create a name-domain mismatch with the raw
    // Schema). Only CircularDependencyResolution runs.
    let semantic = schema_codegen::Normalizer::new()
        .normalize_with_pipeline(
            &schema,
            schema_codegen::PipelineBuilder::new()
                .consolidation(schema_codegen::Consolidation::Skip)
                .naming(schema_codegen::Naming::Skip)
                .build(),
        )
        .map_err(|errors| {
            anyhow::anyhow!(
                "Schema normalization failed: {}",
                errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

    let mut generated_code = Vec::new();

    // Generate file header
    let file_header = templates::FileHeader {
        schema_name: schema.name.clone(),
        schema_description: schema.description.clone(),
    };
    generated_code.push(file_header.render());
    // Use the semantic IR for import detection — iterate types once,
    // matching on SemanticType which provides the fully resolved view.
    let has_enums = semantic
        .types()
        .any(|t| matches!(t, schema_codegen::SemanticType::Enum(_)));

    let (has_literal, has_discriminated_unions, has_externally_tagged_enums) = {
        let mut has_literal = false;
        let mut has_discriminated_unions = false;
        let mut has_externally_tagged_enums = false;

        for sem_type in semantic.types() {
            if let schema_codegen::SemanticType::Enum(sem_enum) = sem_type {
                match &sem_enum.representation {
                    reflectapi_schema::Representation::Internal { .. }
                    | reflectapi_schema::Representation::Adjacent { .. } => {
                        // Both internally and adjacently tagged enums now use
                        // Literal discriminator fields + Pydantic discriminated unions
                        has_literal = true;
                        has_discriminated_unions = true;
                    }
                    reflectapi_schema::Representation::External => {
                        let has_complex_variants = sem_enum
                            .variants
                            .values()
                            .any(|v| !matches!(v.field_style, schema_codegen::FieldStyle::Unit));
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

    // Use semantic IR for function introspection
    let has_warnings = semantic.functions().any(|f| f.deprecation_note.is_some());
    let has_streaming = schema
        .functions()
        .any(|f| matches!(f.output_type, OutputType::Stream { .. }));
    let has_partial_models = schema_has_partial_field(&schema);
    let python_metadata = collect_python_metadata_usage(&all_type_names);

    // Generate imports
    let imports = templates::Imports {
        has_async: config.generate_async,
        has_sync: config.generate_sync,
        has_testing: config.generate_testing,
        has_enums,
        has_warnings,
        extra_stdlib_imports: python_metadata.stdlib_imports.iter().cloned().collect(),
        extra_runtime_imports: python_metadata.runtime_imports.iter().cloned().collect(),
        has_generics: true,
        has_annotated: true, // Always include for external type fallbacks
        has_literal,
        has_discriminated_unions,
        has_externally_tagged_enums,
        global_type_vars: Vec::new(), // Will be added later after tracking usage
        has_streaming,
        has_partial_models,
    };
    // Use optimized import generation instead of template
    generated_code.push(generate_optimized_imports(&imports));

    // Types that are provided by the runtime library and should not be generated
    let mut non_rendered_types = python_metadata.runtime_provided_types.clone();
    non_rendered_types.insert("std::option::Option".to_string());

    let class_names = build_python_class_name_map(
        semantic
            .types()
            .filter(|st| !non_rendered_types.contains(st.name()))
            .map(|st| st.name()),
    );

    // Build the set of Python class names that will be emitted, so we can
    // detect TypeVar-vs-class name collisions below.
    let emitted_class_names: HashSet<String> = class_names.values().cloned().collect();

    // Collect TypeVars used across all types, using semantic IR ordering.
    // Since the codegen pipeline skips NamingResolution, sem_type.name()
    // matches the raw Schema's type names exactly — no original_name bridging needed.
    let mut used_type_vars: BTreeSet<String> = BTreeSet::new();
    for sem_type in semantic.types() {
        let type_name = sem_type.name();
        if non_rendered_types.contains(type_name) {
            continue;
        }
        let type_def = match schema.get_type(type_name) {
            Some(t) => t,
            None => continue,
        };

        // Collect TypeVars from this type
        collect_type_vars_from_type(type_def, &schema, &implemented_types, &mut used_type_vars)?;
    }

    // Build a rename map for TypeVars that collide with class names.
    // When a Rust type parameter has the same name as a top-level type
    // (e.g., `Identity` is both a type parameter and a struct), the class
    // definition would overwrite the TypeVar. We prefix these with `_T_`
    // so the TypeVar and class can coexist in Python's single namespace.
    let typevar_rename_map: BTreeMap<String, String> = used_type_vars
        .iter()
        .filter(|tv| emitted_class_names.contains(tv.as_str()))
        .map(|tv| (tv.clone(), format!("_T_{tv}")))
        .collect();

    // Apply the rename map to the schema's type parameter names and type
    // references. This ensures all downstream rendering functions
    // automatically use the renamed TypeVar names without needing to
    // thread a rename map through every function signature.
    if !typevar_rename_map.is_empty() {
        rename_type_params_in_schema(&mut schema, &typevar_rename_map);
    }

    // Apply renames to the used_type_vars set
    let renamed_type_vars: BTreeSet<String> = used_type_vars
        .iter()
        .map(|tv| {
            typevar_rename_map
                .get(tv)
                .cloned()
                .unwrap_or_else(|| tv.clone())
        })
        .collect();

    // Generate TypeVar declarations
    if !renamed_type_vars.is_empty() {
        generated_code.push("".to_string());
        generated_code.push("# Type variables for generic types".to_string());
        generated_code.push("".to_string());
        for type_var in &renamed_type_vars {
            generated_code.push(format!("{type_var} = TypeVar(\"{type_var}\")"));
        }
        generated_code.push("".to_string());
    }

    // Render all types (models and enums)
    let mut rendered_types = BTreeMap::new();

    // Use SemanticSchema for type ordering — it provides deterministic
    // BTreeMap ordering from the Normalizer's dependency analysis.
    // Names match the raw Schema directly (no NamingResolution was applied).
    let mut rendered_type_names_in_order: Vec<String> = Vec::new();

    for sem_type in semantic.types() {
        let type_name = sem_type.name().to_string();

        if non_rendered_types.contains(&type_name) {
            continue;
        }

        let type_def = match schema.get_type(&type_name) {
            Some(t) => t,
            None => {
                continue;
            }
        };

        // TypeVars have already been collected, use empty set for rendering
        let mut dummy_type_vars = BTreeSet::new();
        let rendered = render_type(
            type_def,
            &schema,
            &implemented_types,
            &class_names,
            &mut dummy_type_vars,
        )?;

        // Only store non-empty renders (excludes unwrapped tuple structs)
        if !rendered.trim().is_empty() {
            rendered_types.insert(type_name.clone(), rendered);
            rendered_type_names_in_order.push(type_name);
        }
    }

    for (type_name, rendered) in
        render_concrete_flattened_generic_structs(&schema, &implemented_types, &class_names)?
    {
        rendered_types.insert(type_name.clone(), rendered);
        rendered_type_names_in_order.push(type_name);
    }

    // Collect rendered type keys before passing ownership to the module tree builder.
    let rendered_type_keys: Vec<String> = rendered_types.keys().cloned().collect();

    // Build namespace module tree. Single-file output still renders this tree
    // as aliases; multi-file output writes each node as a real Python module.
    let module_tree = modules_from_rendered_types(rendered_type_names_in_order, rendered_types);

    // TypeVar declarations are now generated at the top of the file (after imports)

    // Generate client class with nested method organization.
    // Use SemanticSchema for deterministic function ordering, but
    // look up raw Function objects for rendering (render_function
    // needs the raw TypeReference fields).
    let raw_functions_by_name: BTreeMap<String, &Function> =
        schema.functions().map(|f| (f.name.clone(), f)).collect();

    let mut function_groups: BTreeMap<String, Vec<templates::Function>> = BTreeMap::new();
    let mut top_level_functions: Vec<templates::Function> = Vec::new();

    for sem_func in semantic.functions() {
        let function_schema = match raw_functions_by_name.get(&sem_func.name) {
            Some(f) => f,
            None => continue,
        };
        let rendered_function = render_function(function_schema, &schema, &implemented_types)?;

        // Check for grouping patterns: underscore or dot notation
        if let Some(separator_pos) = sem_func.name.find('_').or_else(|| sem_func.name.find('.')) {
            let group_name = &sem_func.name[..separator_pos];
            let method_name = &sem_func.name[separator_pos + 1..];

            // Create a modified function with the shortened name for nested access
            let mut nested_function = rendered_function.clone();
            nested_function.name =
                safe_python_identifier_with_context(method_name, IdentifierContext::Method);
            nested_function.original_name = Some(rendered_function.name.clone());

            function_groups
                .entry(safe_python_identifier(group_name))
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
    let client_code = client_template.render();

    // Rebuild models in a loop — per-type try/except so one failure
    // doesn't skip all rebuilds. Use flat class names, not namespace-dotted
    // refs, since these are module-level definitions.
    let rebuild_models: Vec<String> = rendered_type_keys
        .iter()
        .filter(|n| !n.starts_with("std::") && !n.starts_with("reflectapi::"))
        .map(|n| python_class_name(n, &class_names))
        .collect();
    // Generate testing utilities if requested
    let testing_code = if config.generate_testing {
        // contains user-defined types that have Pydantic classes generated for them.
        // Note types with fallbacks to primitives are not added.
        let mut user_defined_types: Vec<String> = rendered_type_keys
            .iter()
            .map(|original_name| type_name_to_python_ref(original_name))
            .collect();
        user_defined_types.sort();

        let testing_template = templates::TestingModule {
            types: user_defined_types,
        };
        Some(testing_template.render())
    } else {
        None
    };

    Ok(PythonGeneration {
        preamble: generated_code.join("\n\n"),
        schema_name: schema.name.clone(),
        schema_description: schema.description.clone(),
        module_tree,
        rebuild_models,
        client_code,
        testing_code,
    })
}

pub fn generate(schema: Schema, config: &Config) -> anyhow::Result<String> {
    let generation = build_python_generation(schema, config)?;
    let mut generated_code = Vec::new();
    generated_code.push(generation.preamble);

    let module_tree_code = generation.module_tree.render();
    if !module_tree_code.trim().is_empty() {
        generated_code.push(module_tree_code);
    }

    generated_code.push(generation.client_code);
    generated_code.push(render_external_types_and_rebuilds(
        &generation.rebuild_models,
    ));

    if let Some(testing_code) = generation.testing_code {
        generated_code.push(testing_code);
    }

    let result = generated_code.join("\n\n");

    if config.format {
        format_python_code(&result)
    } else {
        Ok(basic_python_format(&result))
    }
}

fn render_external_type_aliases() -> String {
    [
        "# External type definitions",
        "StdNumNonZeroU32 = Annotated[int, \"Rust NonZero u32 type\"]",
        "StdNumNonZeroU64 = Annotated[int, \"Rust NonZero u64 type\"]",
        "StdNumNonZeroI32 = Annotated[int, \"Rust NonZero i32 type\"]",
        "StdNumNonZeroI64 = Annotated[int, \"Rust NonZero i64 type\"]",
        "",
    ]
    .join("\n")
}

fn render_external_types_and_rebuilds(rebuild_models: &[String]) -> String {
    let mut external_types_and_rebuilds = vec![
        render_external_type_aliases(),
        "# Rebuild models to resolve forward references".to_string(),
    ];

    if !rebuild_models.is_empty() {
        // Same strict-rebuild contract as `render_rebuild_file` (see
        // its comment for the rationale).
        external_types_and_rebuilds.push(format!(
            "_rebuild_errors: list[str] = []\n\
             for _model in [\n    {},\n]:\n\
             \x20   if not hasattr(_model, \"model_rebuild\"):\n\
             \x20       continue\n\
             \x20   try:\n\
             \x20       _model.model_rebuild()\n\
             \x20   except Exception as _exc:\n\
             \x20       _rebuild_errors.append(\
             f\"  - {{_model.__name__}}: {{type(_exc).__name__}}: {{_exc}}\")\n\
             if _rebuild_errors:\n\
             \x20   raise RuntimeError(\n\
             \x20       \"reflectapi: failed to rebuild \" + str(len(_rebuild_errors)) + \
             \" generated model(s). This usually means the codegen emitted an annotation \
             pointing at a symbol that was never defined (a dangling type reference). \
             Fix the codegen rather than catching this error.\\n\" + \
             \"\\n\".join(_rebuild_errors)\n\
             \x20   )",
            rebuild_models.join(",\n    ")
        ));
    }
    external_types_and_rebuilds.push("".to_string());
    external_types_and_rebuilds.join("\n")
}

fn safe_python_module_segment(segment: &str) -> String {
    const PYTHON_KEYWORDS: &[&str] = &[
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
        "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return",
        "try", "while", "with", "yield",
    ];

    let mut result: String = segment
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    if result.is_empty() {
        result.push('_');
    }
    if result.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        result.insert(0, '_');
    }
    if PYTHON_KEYWORDS.contains(&result.as_str()) {
        result.push('_');
    }
    result
}

fn extract_defined_python_names(type_code: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in type_code.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("class ") {
            if let Some(paren) = rest.find('(') {
                let name = &rest[..paren];
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            } else if let Some(colon) = rest.find(':') {
                let name = &rest[..colon];
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        } else if let Some(eq_pos) = trimmed.find(" = ") {
            let name = &trimmed[..eq_pos];
            let value = trimmed[eq_pos + 3..].trim();
            if name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && !name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                && !name.ends_with("Variants")
                && name.len() > 2
                && !value.contains("TypeVar")
            {
                names.push(name.to_string());
            }
        }
    }
    names
}

fn flat_namespace_prefix(ns_path: &[String]) -> String {
    ns_path
        .iter()
        .map(|seg| {
            seg.replace("::", "_")
                .split('_')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().chain(chars).collect(),
                    }
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .collect::<Vec<_>>()
        .join("")
}

fn module_aliases(module: &templates::Module, ns_path: &[String]) -> BTreeMap<String, String> {
    let flat_prefix = flat_namespace_prefix(ns_path);
    let mut aliases = BTreeMap::new();

    for mt in &module.types {
        let flat_names = extract_defined_python_names(&mt.rendered);
        // Heuristic: the "primary" flat name for this type is the
        // class whose name matches the rust leaf when both pass through
        // the same PascalCase pipeline. We prefer it for the Rust-leaf
        // alias below. If we can't identify a primary (e.g. variant
        // models that don't carry their own rust path), fall back to
        // emitting only the prefix-stripped alias.
        let rust_leaf = mt.rust_path.rsplit("::").next().unwrap_or("");
        let primary_flat_name = flat_names
            .iter()
            .find(|name| name.ends_with(&to_pascal_case(rust_leaf)))
            .cloned();

        for flat_name in &flat_names {
            // 1. Stripped form: drop the *full* namespace prefix so the
            //    namespace exposes the short ergonomic name. This is
            //    what `__all__` advertises.
            let stripped = if flat_name.starts_with(&flat_prefix) {
                flat_name[flat_prefix.len()..].to_string()
            } else {
                flat_name.clone()
            };
            if !stripped.is_empty() {
                aliases.insert(stripped, flat_name.clone());
            }
        }

        // 2. Rust-leaf form (e.g. `OrderInsertData = MyapiOrderInsertData`).
        //    `type_name_to_python_ref` generates field annotations as
        //    `<namespace>.<rust_leaf>`, so the namespace must also
        //    answer to that name or `model_rebuild()` raises
        //    AttributeError. Only emit when the leaf differs from the
        //    stripped form (otherwise we're duplicating an alias).
        if !rust_leaf.is_empty() {
            let leaf_pascal = improve_class_name_part(rust_leaf);
            if let Some(flat_name) = primary_flat_name {
                if !leaf_pascal.is_empty() && !aliases.contains_key(&leaf_pascal) {
                    aliases.insert(leaf_pascal, flat_name);
                }
            }
        }
    }

    aliases
}

fn collect_flat_imports_by_module(
    module: &templates::Module,
    ns_path: &[String],
    imports: &mut BTreeMap<Vec<String>, BTreeSet<String>>,
) {
    let mut names = BTreeSet::new();
    for mt in &module.types {
        names.extend(extract_defined_python_names(&mt.rendered));
    }
    if !names.is_empty() {
        imports.insert(ns_path.to_vec(), names);
    }

    for sub in module.submodules.values() {
        if sub.is_empty() {
            continue;
        }
        let mut child_path = ns_path.to_vec();
        child_path.push(sub.name.clone());
        collect_flat_imports_by_module(sub, &child_path, imports);
    }
}

fn module_import_path(ns_path: &[String]) -> String {
    ns_path
        .iter()
        .map(|segment| safe_python_module_segment(segment))
        .collect::<Vec<_>>()
        .join(".")
}

fn root_relative_prefix(depth: usize) -> String {
    ".".repeat(depth + 1)
}

fn root_module(ns_path: &[String]) -> String {
    safe_python_module_segment(&ns_path[0])
}

fn relative_module_import_path(ns_path: &[String]) -> String {
    format!(".{}", module_import_path(ns_path))
}

fn module_file_path(ns_path: &[String]) -> String {
    let mut path = ns_path
        .iter()
        .map(|segment| safe_python_module_segment(segment))
        .collect::<Vec<_>>()
        .join("/");
    if !path.is_empty() {
        path.push('/');
    }
    path.push_str("__init__.py");
    path
}

fn render_python_file(config: &Config, code: String) -> anyhow::Result<String> {
    if config.format {
        format_python_code(&code)
    } else {
        Ok(basic_python_format(&code))
    }
}

fn render_module_file(
    generation: &PythonGeneration,
    module: &templates::Module,
    ns_path: &[String],
    root_type_names: &BTreeSet<String>,
    config: &Config,
) -> anyhow::Result<String> {
    let mut parts = vec![generation.preamble.clone(), render_external_type_aliases()];

    if !root_type_names.is_empty() {
        parts.push(format!(
            "from {}_types import {}",
            root_relative_prefix(ns_path.len()),
            root_type_names
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut body = String::new();
    for mt in &module.types {
        body.push_str(&mt.rendered);
        body.push('\n');
    }

    let aliases = module_aliases(module, ns_path);
    if !aliases.is_empty() {
        if !body.trim().is_empty() {
            body.push('\n');
        }
        body.push_str("# Public aliases for this module\n");
        for (leaf, flat_name) in &aliases {
            if leaf != flat_name {
                body.push_str(&format!("{leaf} = {flat_name}\n"));
            }
        }
    }

    let child_modules: Vec<String> = module
        .submodules
        .values()
        .filter(|sub| !sub.is_empty())
        .map(|sub| safe_python_module_segment(&sub.name))
        .collect();
    if !child_modules.is_empty() {
        if !body.trim().is_empty() {
            body.push('\n');
        }
        for child in &child_modules {
            body.push_str(&format!("from . import {child}\n"));
        }
    }

    if ns_path.len() == 1 {
        if !body.trim().is_empty() {
            body.push('\n');
        }
        // Narrow the suppression to `ImportError` only: the wide
        // `except Exception: pass` here used to swallow every
        // `model_rebuild()` failure raised by `_rebuild_models()`,
        // which is the same class of bug we want to surface. The
        // remaining ImportError suppression covers the legitimate case
        // where this namespace is imported standalone before the root
        // package finishes initialising.
        body.push_str("try:\n");
        body.push_str("    from .._rebuild import rebuild_models as _rebuild_models\n\n");
        body.push_str("    _rebuild_models()\n");
        body.push_str("except ImportError:\n");
        body.push_str("    pass\n");
    }

    let mut all_names: BTreeSet<String> = aliases.into_keys().collect();
    for mt in &module.types {
        all_names.extend(extract_defined_python_names(&mt.rendered));
    }
    all_names.extend(child_modules);
    if !all_names.is_empty() {
        body.push_str(&format!(
            "\n__all__ = {:?}\n",
            all_names.into_iter().collect::<Vec<_>>()
        ));
    }

    if !body.trim().is_empty() {
        parts.push(body);
    }

    render_python_file(config, parts.join("\n\n"))
}

fn render_package_module_files(
    generation: &PythonGeneration,
    module: &templates::Module,
    ns_path: &[String],
    root_type_names: &BTreeSet<String>,
    config: &Config,
    files: &mut BTreeMap<String, String>,
) -> anyhow::Result<()> {
    if !ns_path.is_empty() {
        let path = module_file_path(ns_path);
        let previous = files.insert(
            path.clone(),
            render_module_file(generation, module, ns_path, root_type_names, config)?,
        );
        anyhow::ensure!(
            previous.is_none(),
            "Python module path collision while generating {path:?}"
        );
    }

    for sub in module.submodules.values() {
        if sub.is_empty() {
            continue;
        }
        let mut child_path = ns_path.to_vec();
        child_path.push(sub.name.clone());
        render_package_module_files(generation, sub, &child_path, root_type_names, config, files)?;
    }

    Ok(())
}

fn render_rebuild_file(
    generation: &PythonGeneration,
    root_type_names: &BTreeSet<String>,
    flat_imports: &BTreeMap<Vec<String>, BTreeSet<String>>,
    config: &Config,
) -> anyhow::Result<String> {
    let mut lines = vec![templates::FileHeader {
        schema_name: generation.schema_name.clone(),
        schema_description: generation.schema_description.clone(),
    }
    .render()];

    for (ns_path, names) in flat_imports {
        if ns_path.is_empty() {
            continue;
        }
        lines.push(format!("from . import {}", root_module(ns_path)));
        lines.push(format!(
            "from {} import {}",
            relative_module_import_path(ns_path),
            names.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }

    if !root_type_names.is_empty() {
        lines.push("from . import _types".to_string());
        lines.push(format!(
            "from ._types import {}",
            root_type_names
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut body = String::new();
    body.push_str("def rebuild_models() -> None:\n");
    if generation.rebuild_models.is_empty() {
        body.push_str("    return\n");
    } else {
        for ns_path in flat_imports.keys().filter(|path| !path.is_empty()) {
            let module = module_import_path(ns_path);
            let root = safe_python_module_segment(&ns_path[0]);
            body.push_str(&format!("    {module}.{root} = {root}\n"));
        }
        if !root_type_names.is_empty() {
            for root in generation
                .module_tree
                .submodules
                .values()
                .filter(|sub| !sub.is_empty())
                .map(|sub| safe_python_module_segment(&sub.name))
            {
                body.push_str(&format!("    _types.{root} = {root}\n"));
            }
        }
        // Run `model_rebuild()` against every generated model and
        // collect every failure into one structured report. The old
        // shape (`try: model_rebuild() except: pass`) silently
        // swallowed dangling-annotation bugs at import time — they
        // surfaced much later as `AttributeError: module has no
        // attribute X` when a user tried to construct a model. By
        // raising here we catch the whole class of "codegen emitted a
        // dangling reference" bugs the moment the package loads,
        // making CI smoke tests like `python -c 'import api_client'`
        // genuinely informative.
        body.push_str("    errors: list[str] = []\n");
        body.push_str("    for _model in [\n");
        for model in &generation.rebuild_models {
            body.push_str(&format!("        {model},\n"));
        }
        body.push_str("    ]:\n");
        // Some rendered types are Python enums (or other non-Pydantic
        // classes) — they don't have `model_rebuild()` and don't need
        // one. Skip them without flagging an error.
        body.push_str("        if not hasattr(_model, \"model_rebuild\"):\n");
        body.push_str("            continue\n");
        body.push_str("        try:\n");
        body.push_str("            _model.model_rebuild()\n");
        body.push_str("        except Exception as exc:\n");
        body.push_str(
            "            errors.append(f\"  - {_model.__name__}: {type(exc).__name__}: {exc}\")\n",
        );
        body.push_str("    if errors:\n");
        body.push_str(
            "        raise RuntimeError(\n            \
             \"reflectapi: failed to rebuild \" + str(len(errors)) + \
             \" generated model(s). This usually means the codegen \
             emitted an annotation pointing at a symbol that was never \
             defined (a dangling type reference). Fix the codegen \
             rather than catching this error.\\n\" + \"\\n\".join(errors)\n        )\n",
        );
    }
    lines.push(body);

    render_python_file(config, lines.join("\n\n"))
}

fn render_client_file(
    generation: &PythonGeneration,
    root_type_names: &BTreeSet<String>,
    config: &Config,
) -> anyhow::Result<String> {
    let mut parts = vec![generation.preamble.clone()];

    for root in generation
        .module_tree
        .submodules
        .values()
        .filter(|sub| !sub.is_empty())
        .map(|sub| safe_python_module_segment(&sub.name))
    {
        parts.push(format!("from . import {root}"));
    }

    if !root_type_names.is_empty() {
        parts.push(format!(
            "from ._types import {}",
            root_type_names
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    parts.push(
        "from ._rebuild import rebuild_models as _rebuild_models\n\n_rebuild_models()".to_string(),
    );
    parts.push(generation.client_code.clone());
    render_python_file(config, parts.join("\n\n"))
}

fn render_generated_facade(
    generation: &PythonGeneration,
    flat_imports: &BTreeMap<Vec<String>, BTreeSet<String>>,
    root_type_names: &BTreeSet<String>,
    config: &Config,
) -> anyhow::Result<String> {
    let mut lines = vec![
        templates::FileHeader {
            schema_name: generation.schema_name.clone(),
            schema_description: generation.schema_description.clone(),
        }
        .render(),
        "from ._client import AsyncClient".to_string(),
    ];

    if config.generate_sync {
        lines.push("from ._client import Client".to_string());
    }

    let mut all_names = BTreeSet::new();
    all_names.insert("AsyncClient".to_string());
    if config.generate_sync {
        all_names.insert("Client".to_string());
    }

    for (ns_path, names) in flat_imports {
        if ns_path.is_empty() {
            continue;
        }
        lines.push(format!("from . import {}", root_module(ns_path)));
        lines.push(format!(
            "from {} import {}",
            relative_module_import_path(ns_path),
            names.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
        all_names.extend(names.iter().cloned());
    }

    if !root_type_names.is_empty() {
        lines.push(format!(
            "from ._types import {}",
            root_type_names
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
        all_names.extend(root_type_names.iter().cloned());
    }

    lines.push("from ._rebuild import rebuild_models as _rebuild_models".to_string());
    lines.push("_rebuild_models()".to_string());
    lines.push(format!(
        "__all__ = {:?}",
        all_names.into_iter().collect::<Vec<_>>()
    ));

    render_python_file(config, lines.join("\n\n"))
}

fn render_root_init_py(generation: &PythonGeneration, config: &Config) -> anyhow::Result<String> {
    let mut imports = vec!["AsyncClient".to_string()];
    if config.generate_sync {
        imports.push("Client".to_string());
    }

    let mut lines = vec![
        templates::header_docstring(&generation.schema_name, &generation.schema_description),
        format!("from ._client import {}", imports.join(", ")),
    ];

    for root in generation
        .module_tree
        .submodules
        .values()
        .filter(|sub| !sub.is_empty())
        .map(|sub| safe_python_module_segment(&sub.name))
    {
        lines.push(format!("from . import {root}"));
        imports.push(root);
    }

    lines.push("from ._rebuild import rebuild_models as _rebuild_models".to_string());
    lines.push("_rebuild_models()".to_string());
    lines.push(format!("__all__ = {imports:?}"));

    render_python_file(config, lines.join("\n\n"))
}

fn render_package_files(
    generation: PythonGeneration,
    config: &Config,
) -> anyhow::Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    let mut flat_imports = BTreeMap::new();
    collect_flat_imports_by_module(&generation.module_tree, &[], &mut flat_imports);

    let root_key = Vec::<String>::new();
    let root_type_names = flat_imports.remove(&root_key).unwrap_or_default();

    if !root_type_names.is_empty() {
        let root_types_code = generation
            .module_tree
            .types
            .iter()
            .map(|mt| mt.rendered.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        files.insert(
            "_types.py".to_string(),
            render_python_file(
                config,
                [
                    generation.preamble.clone(),
                    render_external_type_aliases(),
                    root_types_code,
                ]
                .join("\n\n"),
            )?,
        );
    }

    render_package_module_files(
        &generation,
        &generation.module_tree,
        &[],
        &root_type_names,
        config,
        &mut files,
    )?;

    files.insert(
        "_client.py".to_string(),
        render_client_file(&generation, &root_type_names, config)?,
    );
    files.insert(
        "_rebuild.py".to_string(),
        render_rebuild_file(&generation, &root_type_names, &flat_imports, config)?,
    );
    files.insert(
        "generated.py".to_string(),
        render_generated_facade(&generation, &flat_imports, &root_type_names, config)?,
    );
    files.insert(
        "__init__.py".to_string(),
        render_root_init_py(&generation, config)?,
    );

    if let Some(testing_code) = &generation.testing_code {
        files.insert(
            "testing.py".to_string(),
            render_python_file(
                config,
                [generation.preamble.clone(), testing_code.clone()].join("\n\n"),
            )?,
        );
    }

    Ok(files)
}

/// Rename type parameters in the schema to avoid TypeVar/class name collisions.
///
/// When a Rust type parameter (e.g., `Identity`) has the same name as a
/// top-level type, the Python codegen would produce both a TypeVar and a
/// class with the same name, causing the class to shadow the TypeVar.
///
/// This function renames the parameters in-place in the schema's type
/// definitions and updates all type references that point to those
/// parameters, so all downstream rendering automatically uses the safe names.
fn rename_type_params_in_schema(schema: &mut Schema, rename_map: &BTreeMap<String, String>) {
    // Process both input_types and output_types since consolidate_types()
    // keeps non-conflicting types in their original typespace.
    let input_type_names: Vec<String> = schema
        .input_types
        .types()
        .map(|t| t.name().to_string())
        .collect();
    for type_name in input_type_names {
        if let Some(type_def) = schema.input_types.get_type_mut(&type_name) {
            rename_type_params_in_type(type_def, rename_map);
        }
    }

    let output_type_names: Vec<String> = schema
        .output_types
        .types()
        .map(|t| t.name().to_string())
        .collect();
    for type_name in output_type_names {
        if let Some(type_def) = schema.output_types.get_type_mut(&type_name) {
            rename_type_params_in_type(type_def, rename_map);
        }
    }
}

/// Rename type parameters within a single type definition.
fn rename_type_params_in_type(
    type_def: &mut reflectapi_schema::Type,
    rename_map: &BTreeMap<String, String>,
) {
    match type_def {
        reflectapi_schema::Type::Struct(s) => {
            // Check if this struct has any parameters that need renaming
            let has_renames = s
                .parameters
                .iter()
                .any(|p| rename_map.contains_key(&p.name));
            if !has_renames {
                return;
            }
            // Rename parameters
            for param in &mut s.parameters {
                if let Some(new_name) = rename_map.get(&param.name) {
                    param.name = new_name.clone();
                }
            }
            // Rename type references in fields
            match &mut s.fields {
                reflectapi_schema::Fields::Named(fields)
                | reflectapi_schema::Fields::Unnamed(fields) => {
                    for field in fields {
                        rename_type_ref(&mut field.type_ref, rename_map);
                    }
                }
                reflectapi_schema::Fields::None => {}
            }
        }
        reflectapi_schema::Type::Enum(e) => {
            let has_renames = e
                .parameters
                .iter()
                .any(|p| rename_map.contains_key(&p.name));
            if !has_renames {
                return;
            }
            for param in &mut e.parameters {
                if let Some(new_name) = rename_map.get(&param.name) {
                    param.name = new_name.clone();
                }
            }
            for variant in &mut e.variants {
                match &mut variant.fields {
                    reflectapi_schema::Fields::Named(fields)
                    | reflectapi_schema::Fields::Unnamed(fields) => {
                        for field in fields {
                            rename_type_ref(&mut field.type_ref, rename_map);
                        }
                    }
                    reflectapi_schema::Fields::None => {}
                }
            }
        }
        reflectapi_schema::Type::Primitive(p) => {
            let has_renames = p
                .parameters
                .iter()
                .any(|pp| rename_map.contains_key(&pp.name));
            if !has_renames {
                return;
            }
            for param in &mut p.parameters {
                if let Some(new_name) = rename_map.get(&param.name) {
                    param.name = new_name.clone();
                }
            }
            if let Some(fallback) = &mut p.fallback {
                rename_type_ref(fallback, rename_map);
            }
        }
    }
}

/// Recursively rename type parameter references in a TypeReference tree.
///
/// Only renames bare references (no generic arguments) that match the rename
/// map, since type parameters are always leaf references with no arguments.
/// A reference like `Identity` with arguments would be a concrete type
/// instantiation, not a type parameter usage.
fn rename_type_ref(type_ref: &mut TypeReference, rename_map: &BTreeMap<String, String>) {
    // Only rename bare names (no arguments) — these are type parameter usages.
    // References with arguments are concrete type instantiations (e.g., Vec<T>).
    if type_ref.arguments.is_empty() {
        if let Some(new_name) = rename_map.get(&type_ref.name) {
            type_ref.name = new_name.clone();
        }
    }
    for arg in &mut type_ref.arguments {
        rename_type_ref(arg, rename_map);
    }
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

fn render_type(
    type_def: &Type,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    match type_def {
        Type::Struct(s) => render_struct(s, schema, implemented_types, class_names, used_type_vars),
        Type::Enum(e) => render_enum(e, schema, implemented_types, class_names, used_type_vars),
        Type::Primitive(_p) => {
            // Primitive types are handled by implemented_types mapping
            Ok(String::new()) // This shouldn't be reached normally
        }
    }
}

/// Recursively collect fields from flattened structures and enums.
/// `field_name` is the original Rust field name for the flattened field
/// (used when emitting enum types as regular fields).
#[allow(clippy::too_many_arguments)]
fn collect_flattened_fields(
    type_ref: &TypeReference,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
    parent_required: bool,
    depth: usize,
    used_type_vars: &mut BTreeSet<String>,
    field_name: Option<&str>,
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

            is_partial: false,
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

    match schema.get_type(target_type_name) {
        Some(reflectapi_schema::Type::Struct(struct_def)) => {
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
                        Some(field.name()),
                    )?;
                    collected_fields.extend(nested_fields);
                } else {
                    // Regular field in flattened struct
                    collected_fields.push(make_flattened_field(
                        field,
                        schema,
                        implemented_types,
                        active_generics,
                        parent_required,
                        depth,
                        used_type_vars,
                    )?);
                }
            }
        }
        Some(reflectapi_schema::Type::Enum(enum_def)) => {
            // Flattened enum: expand variant fields into the parent model.
            // For internally-tagged enums, each variant's fields + the tag field
            // get merged into the parent struct. For other representations,
            // we emit the enum as a regular typed field since Pydantic cannot
            // truly flatten non-internally-tagged unions.
            collect_flattened_enum_fields(
                enum_def,
                type_ref,
                schema,
                implemented_types,
                active_generics,
                parent_required,
                used_type_vars,
                &mut collected_fields,
                field_name,
            )?;
        }
        Some(reflectapi_schema::Type::Primitive(p)) => {
            // Unit-like primitives (e.g. std::tuple::Tuple0) flatten to
            // nothing — that's legitimate. Other primitives in a
            // flatten position are a schema error: you can't flatten an
            // i32 etc.
            if p.name != "std::tuple::Tuple0" {
                anyhow::bail!(
                    "python codegen: cannot flatten primitive type '{}' \
                     (only structs, internally-tagged enums, and unit types \
                     are valid flatten targets)",
                    p.name,
                );
            }
        }
        None => {
            // We resolved a flatten target by name and the type wasn't
            // in the schema. After monomorphization, this should be
            // impossible — generic parameters are substituted away
            // before rendering. Bail rather than silently drop the
            // inner type's fields (the bug fixed by monomorphization).
            anyhow::bail!(
                "python codegen: cannot resolve flatten target '{target_type_name}'. \
                 If this is a generic parameter, monomorphization should \
                 have replaced it; this is a codegen bug.",
            );
        }
    }

    Ok(collected_fields)
}

/// Create a single flattened field entry from a struct field
fn make_flattened_field(
    field: &reflectapi_schema::Field,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
    parent_required: bool,
    depth: usize,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<templates::Field> {
    let field_type = type_ref_to_python_type(
        &field.type_ref,
        schema,
        implemented_types,
        active_generics,
        used_type_vars,
    )?;

    let (optional, default_value, final_field_type) = resolve_field_optionality(
        &field.type_ref.name,
        field_type,
        field.required && parent_required,
    );

    let (sanitized, alias) = sanitize_field_name_with_alias(field.name(), field.serde_name());
    Ok(templates::Field {
        name: sanitized,
        type_annotation: final_field_type,
        description: Some(format!(
            "(flattened{}) {}",
            if depth > 1 {
                format!(" depth={depth}")
            } else {
                String::new()
            },
            field.description()
        )),
        deprecation_note: field.deprecation_note.clone(),
        optional,
        default_value,
        alias,

        is_partial: field.type_ref.name == "reflectapi::Option",
    })
}

/// Handle flattened enum fields by emitting the enum as a typed field.
///
/// For internally-tagged enums (`#[serde(tag = "...")]`), serde merges the
/// tag + variant fields into the parent struct. Pydantic can't directly
/// represent this, so we emit the enum's Python union type as a regular field.
/// The model uses `extra="allow"` to accept the flattened wire format.
#[allow(clippy::too_many_arguments)]
fn collect_flattened_enum_fields(
    enum_def: &reflectapi_schema::Enum,
    type_ref: &TypeReference,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    active_generics: &[String],
    parent_required: bool,
    used_type_vars: &mut BTreeSet<String>,
    collected_fields: &mut Vec<templates::Field>,
    original_field_name: Option<&str>,
) -> anyhow::Result<()> {
    let enum_python_type = type_ref_to_python_type(
        type_ref,
        schema,
        implemented_types,
        active_generics,
        used_type_vars,
    )?;

    // Use the original Rust field name if available, otherwise derive from type name
    let field_name = original_field_name
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            type_ref
                .name
                .split("::")
                .last()
                .unwrap_or(&type_ref.name)
                .to_lowercase()
        });
    let (sanitized, alias) = sanitize_field_name_with_alias(&field_name, &field_name);

    let (optional, default_value, final_type) = if !parent_required {
        (
            true,
            Some("None".to_string()),
            format!("{enum_python_type} | None"),
        )
    } else {
        (false, None, enum_python_type)
    };

    collected_fields.push(templates::Field {
        name: sanitized,
        type_annotation: final_type,
        description: Some(format!("(flattened enum: {})", enum_def.name)),
        deprecation_note: None,
        optional,
        default_value,
        alias,

        is_partial: false,
    });

    Ok(())
}

fn render_struct(
    struct_def: &reflectapi_schema::Struct,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    // Check if this struct has any flattened fields
    let has_flattened = struct_def.fields.iter().any(|field| field.flattened());

    if has_flattened {
        // Use runtime flatten support
        return render_struct_with_flatten(
            struct_def,
            schema,
            implemented_types,
            class_names,
            used_type_vars,
        );
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

            let (optional, default_value, field_type) =
                resolve_field_optionality(&field.type_ref.name, base_field_type, field.required);

            let (sanitized, alias) =
                sanitize_field_name_with_alias(field.name(), field.serde_name());
            Ok(templates::Field {
                name: sanitized,
                type_annotation: field_type,
                description: Some(field.description().to_string()),
                deprecation_note: field.deprecation_note.clone(),
                optional,
                default_value,
                alias,

                is_partial: field.type_ref.name == "reflectapi::Option",
            })
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    // Note: flattened fields are handled by render_struct_with_flatten (early return
    // at the top of this function). This path only handles non-flattened structs.
    let all_fields = regular_fields;

    // Check if this is a generic struct (has type parameters)
    let has_generics = !struct_def.parameters.is_empty();
    let class_name = python_class_name(&struct_def.name, class_names);

    let class_template = templates::DataClass {
        name: class_name,
        description: Some(struct_def.description().to_string()),
        fields: all_fields,
        is_tuple: false,
        is_generic: has_generics,
        generic_params: active_generics.clone(),
    };

    Ok(class_template.render())
}

fn render_enum(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::{Fields, Representation};

    // Check if this is a tagged enum (internally tagged)
    match &enum_def.representation {
        Representation::Internal { tag } => {
            let (rendered, _union_variant_names) = render_internally_tagged_enum(
                enum_def,
                tag,
                schema,
                implemented_types,
                class_names,
                used_type_vars,
            )?;

            Ok(rendered)
        }
        Representation::Adjacent { tag, content } => {
            // Adjacently tagged enums are represented as { tag: "Variant", content: ... }
            let (rendered, _union_variant_names) = render_adjacently_tagged_enum(
                enum_def,
                tag,
                content,
                schema,
                implemented_types,
                class_names,
                used_type_vars,
            )?;

            Ok(rendered)
        }
        Representation::None => {
            // Untagged enums
            render_untagged_enum(
                enum_def,
                schema,
                implemented_types,
                class_names,
                used_type_vars,
            )
        }
        _ => {
            // Check if this is a primitive-represented enum (has discriminant values)
            let has_discriminants = enum_def.variants.iter().any(|v| v.discriminant.is_some());

            if has_discriminants {
                render_primitive_enum(enum_def, class_names)
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
                    // This is an externally tagged enum with complex variants
                    render_externally_tagged_enum(
                        enum_def,
                        schema,
                        implemented_types,
                        class_names,
                        used_type_vars,
                    )
                } else {
                    // Simple string enum
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
                        name: python_class_name(&enum_def.name, class_names),
                        description: Some(enum_def.description().to_string()),
                        variants,
                    };

                    Ok(enum_template.render())
                }
            }
        }
    }
}

fn render_adjacently_tagged_enum(
    enum_def: &reflectapi_schema::Enum,
    tag: &str,
    content: &str,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<(String, Vec<String>)> {
    use reflectapi_schema::Fields;

    let enum_name = python_class_name(&enum_def.name, class_names);

    let mut variant_class_definitions: Vec<String> = Vec::new();
    let mut union_variant_names: Vec<String> = Vec::new();
    let generic_params: Vec<String> = infer_enum_generic_params(enum_def, schema);

    // Track used generic type variables
    for generic in &generic_params {
        used_type_vars.insert(generic.clone());
    }

    let is_generic = !generic_params.is_empty();

    // Generate individual variant classes, each with the tag field as Literal discriminator
    for variant in &enum_def.variants {
        let variant_class_name = format!("{}{}", enum_name, to_pascal_case(variant.name()));

        // For generic enums, add type parameters to union variant names
        if is_generic {
            let params_str = generic_params.join(", ");
            union_variant_names.push(format!("{variant_class_name}[{params_str}]"));
        } else {
            union_variant_names.push(variant_class_name.clone());
        }

        // Start with the tag discriminator field
        let discriminator_default_value = Some(format!("\"{}\"", variant.serde_name()));
        let mut fields = vec![templates::Field {
            name: tag.to_string(),
            type_annotation: format!("Literal['{}']", variant.serde_name()),
            description: Some("Discriminator field".to_string()),
            deprecation_note: None,
            optional: false,
            default_value: discriminator_default_value,
            alias: None,

            is_partial: false,
        }];

        // Add the content field based on variant type
        match &variant.fields {
            Fields::None => {
                // Unit variant: only the tag field, no content field needed.
                // The content field is optional and absent on the wire.
            }
            Fields::Unnamed(unnamed_fields) => {
                // Tuple variant: add content field with the appropriate type
                if unnamed_fields.len() == 1 {
                    let field_type = type_ref_to_python_type(
                        &unnamed_fields[0].type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;
                    fields.push(templates::Field {
                        name: content.to_string(),
                        type_annotation: field_type,
                        description: Some(unnamed_fields[0].description().to_string()),
                        deprecation_note: unnamed_fields[0].deprecation_note.clone(),
                        optional: false,
                        default_value: None,
                        alias: None,

                        is_partial: false,
                    });
                } else {
                    // Multi-field tuple: content is a list
                    // Resolve types for side-effects (populates used_type_vars)
                    for f in unnamed_fields {
                        type_ref_to_python_type(
                            &f.type_ref,
                            schema,
                            implemented_types,
                            &generic_params,
                            used_type_vars,
                        )?;
                    }
                    fields.push(templates::Field {
                        name: content.to_string(),
                        type_annotation: "list[Any]".to_string(),
                        description: Some("Tuple variant fields".to_string()),
                        deprecation_note: None,
                        optional: false,
                        default_value: None,
                        alias: None,

                        is_partial: false,
                    });
                }
            }
            Fields::Named(named_fields) => {
                // Struct variant: we need a nested content model, then reference it
                // First generate the content model
                let content_class_name = format!("{variant_class_name}Content");
                let mut content_fields = Vec::new();
                for field in named_fields {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;
                    let (optional, default_value, final_field_type) =
                        resolve_field_optionality(&field.type_ref.name, field_type, field.required);
                    let (sanitized, alias) =
                        sanitize_field_name_with_alias(field.name(), field.serde_name());
                    content_fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,

                        is_partial: field.type_ref.name == "reflectapi::Option",
                    });
                }
                let content_model = templates::DataClass {
                    name: content_class_name.clone(),
                    description: Some(format!("{} content", variant.name())),
                    fields: content_fields,
                    is_tuple: false,
                    is_generic,
                    generic_params: generic_params.clone(),
                };
                variant_class_definitions.push(content_model.render());

                // Reference the content model in the variant class
                let content_type = if is_generic {
                    format!("{}[{}]", content_class_name, generic_params.join(", "))
                } else {
                    content_class_name
                };
                fields.push(templates::Field {
                    name: content.to_string(),
                    type_annotation: content_type,
                    description: Some(format!("{} content", variant.name())),
                    deprecation_note: None,
                    optional: false,
                    default_value: None,
                    alias: None,

                    is_partial: false,
                });
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

        variant_class_definitions.push(variant_template.render());
    }

    // Generate the discriminated union using the same pattern as internally tagged enums
    let union_variants_for_template: Vec<templates::UnionVariant> = enum_def
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
        variants: union_variants_for_template,
        discriminator_field: tag.to_string(),
        is_generic,
        generic_params: generic_params.clone(),
    };

    let union_definition = union_template.render();

    // Combine all parts
    let mut result = String::new();

    // Add variant class definitions (content models + variant models)
    result.push_str(&variant_class_definitions.join("\n\n"));
    if !result.is_empty() {
        result.push_str("\n\n");
    }

    // Add union definition
    result.push_str(&union_definition);
    result.push_str("\n\n");

    Ok((result, union_variant_names))
}

fn render_externally_tagged_enum(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let enum_name = python_class_name(&enum_def.name, class_names);
    let mut variant_models = Vec::new();
    let mut union_variants = Vec::new();

    // Collect per-variant data for the compact helper-based approach
    let mut variant_entries = Vec::new(); // (wire_name, handler_expr)
    let mut serializer_entries = Vec::new(); // (wire_name, check_expr, serialize_expr)
    let mut variant_class_names = Vec::new(); // class names for isinstance checks

    // Collect active generic parameter names for this enum
    let generic_params: Vec<String> = infer_enum_generic_params(enum_def, schema);

    // Track used generic type variables
    for generic in &generic_params {
        used_type_vars.insert(generic.clone());
    }

    // Generate variant models and build validation/serialization logic
    for variant in &enum_def.variants {
        let variant_name = variant.name();
        let wire_name = variant.serde_name();

        match &variant.fields {
            Fields::None => {
                // Unit variant: represented as string literal
                union_variants.push(format!("Literal[{variant_name:?}]"));

                variant_entries.push(format!("\"{wire_name}\": \"_unit\""));
                serializer_entries.push(format!(
                    "\"{wire_name}\": (lambda r: r == \"{wire_name}\", lambda r: \"{wire_name}\")"
                ));
            }
            Fields::Unnamed(unnamed_fields) => {
                // Tuple variant: create a model class
                let variant_class_name =
                    format!("{}{}Variant", enum_name, to_pascal_case(variant_name));
                variant_class_names.push(variant_class_name.clone());
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

                    let field_name = format!("field_{i}");
                    field_names.push(field_name.clone());

                    fields.push(templates::Field {
                        name: field_name,
                        type_annotation: field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional: false,
                        default_value: None,
                        alias: None,

                        is_partial: field.type_ref.name == "reflectapi::Option",
                    });
                }

                let variant_model = templates::DataClass {
                    name: variant_class_name.clone(),
                    description: Some(format!("{variant_name} variant")),
                    fields,
                    is_tuple: true,
                    is_generic: !generic_params.is_empty(),
                    generic_params: generic_params.clone(),
                };

                variant_models.push(variant_model.render());
                let union_member = if !generic_params.is_empty() {
                    format!("{}[{}]", variant_class_name, generic_params.join(", "))
                } else {
                    variant_class_name.clone()
                };
                union_variants.push(union_member);

                // For tuple variants, the JSON value can be a single value or array
                if field_names.len() == 1 {
                    variant_entries.push(format!(
                        "\"{wire_name}\": lambda v: {variant_class_name}(field_0=v)"
                    ));
                    serializer_entries.push(format!(
                        "\"{wire_name}\": (lambda r: isinstance(r, {variant_class_name}), lambda r: {{\"{wire_name}\": r.field_0}})"
                    ));
                } else {
                    let assigns = field_names
                        .iter()
                        .enumerate()
                        .map(|(i, name)| format!("{name}=v[{i}]"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    variant_entries.push(format!(
                        "\"{wire_name}\": lambda v: {variant_class_name}({assigns}) if isinstance(v, list) else (_ for _ in ()).throw(ValueError(\"Expected list for tuple variant {wire_name}\"))"
                    ));
                    let field_accesses = field_names
                        .iter()
                        .map(|name| format!("r.{name}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    serializer_entries.push(format!(
                        "\"{wire_name}\": (lambda r: isinstance(r, {variant_class_name}), lambda r: {{\"{wire_name}\": [{field_accesses}]}})"
                    ));
                }
            }
            Fields::Named(named_fields) => {
                // Struct variant: create a model class
                let variant_class_name =
                    format!("{}{}Variant", enum_name, to_pascal_case(variant_name));
                variant_class_names.push(variant_class_name.clone());
                let mut fields = Vec::new();

                for field in named_fields {
                    let field_type = type_ref_to_python_type(
                        &field.type_ref,
                        schema,
                        implemented_types,
                        &generic_params,
                        used_type_vars,
                    )?;

                    let (optional, default_value, final_field_type) =
                        resolve_field_optionality(&field.type_ref.name, field_type, field.required);

                    let (sanitized, alias) =
                        sanitize_field_name_with_alias(field.name(), field.serde_name());
                    fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,

                        is_partial: field.type_ref.name == "reflectapi::Option",
                    });
                }

                let variant_model = templates::DataClass {
                    name: variant_class_name.clone(),
                    description: Some(format!("{variant_name} variant")),
                    fields,
                    is_tuple: false,
                    is_generic: !generic_params.is_empty(),
                    generic_params: generic_params.clone(),
                };

                variant_models.push(variant_model.render());
                let union_member = if !generic_params.is_empty() {
                    format!("{}[{}]", variant_class_name, generic_params.join(", "))
                } else {
                    variant_class_name.clone()
                };
                union_variants.push(union_member);

                variant_entries.push(format!(
                    "\"{wire_name}\": lambda v: {variant_class_name}(**v)"
                ));
                serializer_entries.push(format!(
                    "\"{wire_name}\": (lambda r: isinstance(r, {variant_class_name}), lambda r: {{\"{wire_name}\": r.model_dump(by_alias=True)}})"
                ));
            }
        }
    }

    // Check if this enum is generic
    let is_generic = !generic_params.is_empty();

    // Render using compact helper-based template
    let template = templates::ExternallyTaggedEnumCompact {
        name: enum_name.clone(),
        description: if enum_def.description().is_empty() {
            None
        } else {
            Some(sanitize_description(enum_def.description()))
        },
        variant_models,
        union_variants: union_variants.join(", "),
        is_single_variant: union_variants.len() == 1,
        variant_entries,
        serializer_entries,
        variant_class_names,
        is_generic,
        generic_params: generic_params.clone(),
    };
    let enum_code = template.render();

    Ok(enum_code)
}

fn render_primitive_enum(
    enum_def: &reflectapi_schema::Enum,
    class_names: &BTreeMap<String, String>,
) -> anyhow::Result<String> {
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
        name: python_class_name(&enum_def.name, class_names),
        description: Some(enum_def.description().to_string()),
        variants: enum_variants,
        is_int_enum: !is_float_enum,
    };

    Ok(enum_template.render())
}

fn render_internally_tagged_enum(
    enum_def: &reflectapi_schema::Enum,
    tag: &str,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<(String, Vec<String>)> {
    use reflectapi_schema::{Fields, Type};

    let enum_name = python_class_name(&enum_def.name, class_names);
    let mut variant_class_definitions: Vec<String> = Vec::new();
    let mut union_variant_names: Vec<String> = Vec::new();

    // Check if this enum is generic
    let generic_params: Vec<String> = infer_enum_generic_params(enum_def, schema);

    // Track used generic type variables
    for generic in &generic_params {
        used_type_vars.insert(generic.clone());
    }

    let is_generic = !generic_params.is_empty();

    // TypeVar definitions are emitted once at the top of the file;
    // inline declarations are suppressed to avoid collisions with class names.

    // Generate individual classes for each variant
    for variant in &enum_def.variants {
        let variant_class_name = format!("{}{}", enum_name, to_pascal_case(variant.name()));

        // For generic enums, add the type parameters to the union variant names
        if is_generic {
            let params_str = generic_params.join(", ");
            union_variant_names.push(format!("{variant_class_name}[{params_str}]"));
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

            is_partial: false,
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
                let inner_type_def = schema
                    .get_type(inner_type_name)
                    .ok_or_else(|| anyhow::anyhow!("Type {inner_type_name} not found in schema"))?;

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
                            if struct_field.flattened() {
                                // Recurse into the flattened type and lift
                                // its fields into this variant; otherwise
                                // the wire shape gets a wrapper layer that
                                // serde never produced.
                                let nested = collect_flattened_fields(
                                    &struct_field.type_ref,
                                    schema,
                                    implemented_types,
                                    &generic_params,
                                    struct_field.required,
                                    0,
                                    used_type_vars,
                                    Some(struct_field.name()),
                                )?;
                                fields.extend(nested);
                                continue;
                            }
                            let field_type = type_ref_to_python_type(
                                &struct_field.type_ref,
                                schema,
                                implemented_types,
                                &generic_params,
                                used_type_vars,
                            )?;

                            let (optional, default_value, final_field_type) =
                                resolve_field_optionality(
                                    &struct_field.type_ref.name,
                                    field_type,
                                    struct_field.required,
                                );

                            let (sanitized, alias) = sanitize_field_name_with_alias(
                                struct_field.name(),
                                struct_field.serde_name(),
                            );
                            fields.push(templates::Field {
                                name: sanitized,
                                type_annotation: final_field_type,
                                description: Some(struct_field.description().to_string()),
                                deprecation_note: struct_field.deprecation_note.clone(),
                                optional,
                                default_value,
                                alias,

                                is_partial: struct_field.type_ref.name == "reflectapi::Option",
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

                            is_partial: inner_field.type_ref.name == "reflectapi::Option",
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

                    let (optional, default_value, final_field_type) =
                        resolve_field_optionality(&field.type_ref.name, field_type, field.required);

                    let (sanitized, alias) =
                        sanitize_field_name_with_alias(field.name(), field.serde_name());
                    fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,

                        is_partial: field.type_ref.name == "reflectapi::Option",
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

        variant_class_definitions.push(variant_template.render());
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

    let union_definition = union_template.render();

    // Combine all parts
    let mut result = String::new();

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
    let header = format!("class {enum_name}(RootModel[{enum_name}Variants])");
    let replacement =
        format!("class {enum_name}(RootModel[{enum_name}Variants]{generic_inherits})");
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

    Ok((result, union_variant_names))
}

fn render_untagged_enum(
    enum_def: &reflectapi_schema::Enum,
    schema: &Schema,
    implemented_types: &BTreeMap<String, String>,
    class_names: &BTreeMap<String, String>,
    used_type_vars: &mut BTreeSet<String>,
) -> anyhow::Result<String> {
    use reflectapi_schema::Fields;

    let enum_name = python_class_name(&enum_def.name, class_names);
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
                    let (optional, default_value, final_field_type) =
                        resolve_field_optionality(&field.type_ref.name, field_type, field.required);

                    let (sanitized, alias) =
                        sanitize_field_name_with_alias(field.name(), field.serde_name());
                    fields.push(templates::Field {
                        name: sanitized,
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias,

                        is_partial: field.type_ref.name == "reflectapi::Option",
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
                    let (optional, default_value, final_field_type) =
                        resolve_field_optionality(&field.type_ref.name, field_type, field.required);

                    fields.push(templates::Field {
                        name: if unnamed_fields.len() == 1 {
                            "value".to_string()
                        } else {
                            format!("field_{i}")
                        },
                        type_annotation: final_field_type,
                        description: Some(field.description().to_string()),
                        deprecation_note: field.deprecation_note.clone(),
                        optional,
                        default_value,
                        alias: None,

                        is_partial: field.type_ref.name == "reflectapi::Option",
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

        variant_classes.push(variant_template.render());
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
    let union_definition = union_template.render();

    // Combine all parts
    let mut result = variant_classes.join("\n\n");
    if !result.is_empty() {
        result.push_str("\n\n");
    }
    result.push_str(&union_definition);

    // Untagged enums - variants serialize directly to their values

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

    let (output_type, stream_item_type) = match &function.output_type {
        OutputType::Complete {
            output_type: Some(output_type),
        } => (
            type_ref_to_python_type_simple(output_type, schema, implemented_types, &[])?,
            None,
        ),
        OutputType::Complete { output_type: None } => ("Any".to_string(), None),
        OutputType::Stream { item_type } => {
            let item_py =
                type_ref_to_python_type_simple(item_type, schema, implemented_types, &[])?;
            (item_py.clone(), Some(item_py))
        }
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
        stream_item_type,
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
                        description: Some(format!("Path parameter: {current_param}")),
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
        result = format!("_{result}");
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
        result = format!("{result}_");
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

/// Produce a unique flat Python class name from a potentially-qualified Rust
/// type name.
///
/// For qualified names (containing `::`), ALL segments are joined into a
/// single PascalCase identifier to guarantee uniqueness across namespaces.
/// For example: `"reflectapi_demo::tests::serde::Offer"` → `"ReflectapiDemoTestsSerdeOffer"`.
///
/// For type *references* (in annotations), use [`type_name_to_python_ref`]
/// instead, which produces a dotted path like `reflectapi_demo.tests.serde.Offer`.
/// Maximum class name length before truncation with hash suffix.
const MAX_CLASS_NAME_LEN: usize = 80;

fn maybe_destutter_pascal_parts(pascal_parts: &[String]) -> String {
    let mut result_parts: Vec<&str> = Vec::new();

    if pascal_parts.len() >= 2 {
        let leaf = pascal_parts.last().unwrap();
        for (i, part) in pascal_parts.iter().enumerate() {
            if i + 1 == pascal_parts.len() {
                result_parts.push(part);
            } else if i + 1 == pascal_parts.len() - 1 && leaf.starts_with(part.as_str()) {
                continue;
            } else {
                result_parts.push(part);
            }
        }
    } else {
        result_parts = pascal_parts.iter().map(|s| s.as_str()).collect();
    }

    result_parts.join("")
}

fn finalize_class_name(original_name: &str, raw: String) -> String {
    if raw.len() > MAX_CLASS_NAME_LEN {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        original_name.hash(&mut hasher);
        let hash_full = format!("{:016x}", hasher.finish());
        let hash = &hash_full[..8];
        let truncated = &raw[..MAX_CLASS_NAME_LEN - 9];
        format!("{truncated}_{hash}")
    } else {
        raw
    }
}

fn build_flat_python_class_name(original_name: &str, remove_stutter: bool) -> String {
    let raw = if original_name.contains("::") {
        let parts: Vec<&str> = original_name.split("::").collect();
        let pascal_parts: Vec<String> = parts.iter().map(|part| to_pascal_case(part)).collect();

        if remove_stutter {
            maybe_destutter_pascal_parts(&pascal_parts)
        } else {
            pascal_parts.join("")
        }
    } else if original_name.contains('.') {
        let pos = original_name.rfind('.').unwrap();
        improve_class_name_part(&original_name[pos + 1..])
    } else {
        improve_class_name_part(original_name)
    };

    finalize_class_name(original_name, raw)
}

fn build_python_class_name_map<'a>(
    type_names: impl IntoIterator<Item = &'a str>,
) -> BTreeMap<String, String> {
    let candidates: Vec<(String, String, String)> = type_names
        .into_iter()
        .map(|type_name| {
            (
                type_name.to_string(),
                build_flat_python_class_name(type_name, false),
                build_flat_python_class_name(type_name, true),
            )
        })
        .collect();

    let mut preferred_counts: BTreeMap<String, usize> = BTreeMap::new();
    for (_, _, preferred_name) in &candidates {
        *preferred_counts.entry(preferred_name.clone()).or_default() += 1;
    }

    candidates
        .into_iter()
        .map(|(type_name, fallback_name, preferred_name)| {
            let chosen_name = if preferred_counts.get(&preferred_name) == Some(&1) {
                preferred_name
            } else {
                fallback_name
            };
            (type_name, chosen_name)
        })
        .collect()
}

fn python_class_name(type_name: &str, class_names: &BTreeMap<String, String>) -> String {
    class_names
        .get(type_name)
        .cloned()
        .unwrap_or_else(|| build_flat_python_class_name(type_name, true))
}

fn improve_class_name(original_name: &str) -> String {
    build_flat_python_class_name(original_name, true)
}

/// Convert a fully-qualified Rust type name to a dotted Python reference.
///
/// Each `::` segment becomes a dot-separated component.  Namespace segments
/// keep their original casing (typically snake_case), while the final leaf
/// segment is run through `improve_class_name_part` (PascalCase).
///
/// Example: `"reflectapi_demo::tests::serde::Offer"` → `"reflectapi_demo.tests.serde.Offer"`
fn type_name_to_python_ref(original_name: &str) -> String {
    let parts: Vec<&str> = original_name.split("::").collect();
    if parts.len() <= 1 {
        // No namespace — just return the PascalCase leaf.
        return improve_class_name(original_name);
    }
    // Namespace segments keep their original casing; the leaf gets PascalCase.
    let namespace_parts = &parts[..parts.len() - 1];
    let leaf = parts.last().unwrap();
    let mut dotted_parts: Vec<String> = namespace_parts
        .iter()
        .map(|p| safe_python_module_segment(p))
        .collect();
    dotted_parts.push(improve_class_name_part(leaf));
    dotted_parts.join(".")
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

fn sanitize_field_name_with_alias(name: &str, serde_name: &str) -> (String, Option<String>) {
    let snake_case = to_snake_case(name);
    let sanitized = safe_python_identifier(&snake_case);

    // Strip leading underscores - Pydantic v2 treats _-prefixed fields as private
    let sanitized = sanitized.trim_start_matches('_').to_string();
    let sanitized = if sanitized.is_empty() {
        // Edge case: name was all underscores
        "field".to_string()
    } else {
        sanitized
    };

    // The wire name (serde_name) is what goes over JSON.
    // We need an alias whenever the Python field name differs from the wire name.
    let wire_name = serde_name;
    if sanitized != wire_name {
        (sanitized, Some(wire_name.to_string()))
    } else {
        (sanitized, None)
    }
}

/// Map external/undefined types to sensible Python equivalents
/// Uses typing.Annotated to provide rich metadata like TypeScript comments
/// Last-resort fallback for types not in the static mapping and not in the schema.
/// Wraps unknown types in Annotated[Any, ...] so the generated code is still valid Python.
fn map_external_type_to_python(type_name: &str) -> String {
    format!("Annotated[Any, \"External type: {type_name}\"]")
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

        if python_type_mappings()
            .get(type_ref.name.as_str())
            .map(|m| m.ignore_type_arguments)
            .unwrap_or(false)
        {
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

    if !type_ref.arguments.is_empty() {
        if let Some(Type::Struct(struct_def)) = schema.get_type(&type_ref.name) {
            if !struct_def.parameters.is_empty()
                && struct_def.fields().any(|field| field.flattened())
            {
                return Ok(concrete_type_ref_name(type_ref));
            }
        }
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

        // Use dotted namespace path for type references (e.g. "mod::Sub::Ty" → "mod.Sub.Ty")
        let base_type = type_name_to_python_ref(&type_ref.name);

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
        "indexmap::IndexMap" => vec!["K", "V"],
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

fn build_implemented_types() -> BTreeMap<String, String> {
    let mut types = BTreeMap::new();

    for (&name, mapping) in python_type_mappings() {
        types.insert(name.to_string(), mapping.type_hint.to_string());
    }

    // Compatibility fallback for legacy/unannotated schemas that use
    // mangled Python class names instead of qualified Rust type names.
    types.insert("StdNumNonZeroU32".to_string(), "int".to_string());
    types.insert("StdNumNonZeroU64".to_string(), "int".to_string());
    types.insert("StdNumNonZeroI32".to_string(), "int".to_string());
    types.insert("StdNumNonZeroI64".to_string(), "int".to_string());
    types.insert("RustDecimalDecimal".to_string(), "str".to_string());

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
    use std::fmt::Write;

    use indexmap::IndexMap;

    /// A tree node representing a Python namespace.
    ///
    /// Types are always rendered at the module top-level (no nesting) to
    /// avoid Python class-scope resolution issues.  The namespace hierarchy
    /// is then rendered as a separate set of namespace `class` wrappers
    /// that contain type aliases pointing to the top-level definitions.
    /// This allows dotted access like `reflectapi_demo.tests.serde.Offer`
    /// while keeping all type definitions in the module globals where
    /// Pydantic can resolve forward-reference annotation strings.
    /// Entry in a Module's type list: the rendered code plus metadata
    /// needed for namespace alias generation.
    pub struct ModuleType {
        /// The rendered Python source code for this type.
        pub rendered: String,
        /// Original fully-qualified Rust type name (e.g.
        /// `myapi::order::OrderInsertData`). Used by the alias
        /// generator to expose the type under both its
        /// flat-prefix-stripped form (`InsertData`) and its Rust-leaf
        /// form (`OrderInsertData`); field annotations elsewhere use
        /// the Rust leaf, so both names must be live in the namespace.
        pub rust_path: String,
    }

    pub struct Module {
        pub name: String,
        pub types: Vec<ModuleType>,
        pub submodules: IndexMap<String, Module>,
    }

    impl Module {
        pub fn is_empty(&self) -> bool {
            self.types.is_empty() && self.submodules.values().all(|m| m.is_empty())
        }

        fn has_namespaces(&self) -> bool {
            !self.submodules.is_empty()
        }

        /// Render this module tree as Python source code.
        ///
        /// Produces two sections:
        /// 1. All type definitions at module top-level (flat)
        /// 2. Namespace class hierarchy with aliases for dotted access
        pub fn render(&self) -> String {
            let mut out = String::new();

            // Part 1: Emit all types at the top level (flat).
            self.collect_types_flat(&mut out);

            // Part 2: Emit namespace class hierarchy with aliases.
            if self.has_namespaces() {
                writeln!(out).unwrap();
                writeln!(out, "# Namespace classes for dotted access to types").unwrap();
                self.render_namespace_aliases(&mut out, 0);
            }

            out
        }

        /// Recursively collect all type definitions from the tree and emit
        /// them at the top level (no indentation wrapping).
        fn collect_types_flat(&self, out: &mut String) {
            for mt in &self.types {
                out.push_str(&mt.rendered);
                out.push('\n');
            }
            for sub in self.submodules.values() {
                sub.collect_types_flat(out);
            }
        }

        /// Extract class and type-alias names defined in rendered type code.
        ///
        /// Looks for patterns like `class FooBar(` or `FooBarVariants = `
        /// at the start of lines.
        fn extract_defined_names(type_code: &str) -> Vec<String> {
            let mut names = Vec::new();
            for line in type_code.lines() {
                // Only match top-level definitions (no leading whitespace)
                // to avoid leaking enum members or nested assignments
                if line.starts_with(' ') || line.starts_with('\t') {
                    continue;
                }
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("class ") {
                    if let Some(paren) = rest.find('(') {
                        let name = &rest[..paren];
                        if !name.is_empty() {
                            names.push(name.to_string());
                        }
                    } else if let Some(colon) = rest.find(':') {
                        let name = &rest[..colon];
                        if !name.is_empty() {
                            names.push(name.to_string());
                        }
                    }
                } else if let Some(eq_pos) = trimmed.find(" = ") {
                    let name = &trimmed[..eq_pos];
                    let value = trimmed[eq_pos + 3..].trim();
                    // Only match PascalCase type aliases (not enum members, constants,
                    // or internal *Variants union aliases).
                    // Skip short names (1-2 chars) which are likely TypeVars, and
                    // also skip any assignment whose value contains `TypeVar`.
                    if name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                        && !name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
                        && !name.ends_with("Variants")
                        && name.len() > 2
                        && !value.contains("TypeVar")
                    {
                        names.push(name.to_string());
                    }
                }
            }
            names
        }

        /// Render namespace alias classes for dotted access.
        ///
        /// `ns_path` accumulates the namespace segments for computing the
        /// flat name prefix that needs to be stripped.
        fn render_namespace_aliases(&self, out: &mut String, indent_level: usize) {
            self.render_namespace_aliases_inner(out, indent_level, &[]);
        }

        fn render_namespace_aliases_inner(
            &self,
            out: &mut String,
            indent_level: usize,
            ns_path: &[&str],
        ) {
            for sub in self.submodules.values() {
                if sub.is_empty() {
                    continue;
                }
                let indent = " ".repeat(indent_level * 4);
                let inner_indent = " ".repeat((indent_level + 1) * 4);

                writeln!(out, "{indent}class {}:", sub.name).unwrap();
                writeln!(
                    out,
                    "{inner_indent}\"\"\"Namespace for {} types.\"\"\"",
                    sub.name
                )
                .unwrap();

                // Compute the flat PascalCase prefix for this namespace level.
                // e.g., for path ["reflectapi_demo", "tests", "serde"], prefix = "ReflectapiDemoTestsSerde"
                let mut full_path: Vec<&str> = ns_path.to_vec();
                full_path.push(&sub.name);
                let flat_prefix: String = full_path
                    .iter()
                    .map(|seg| {
                        // PascalCase each segment (same logic as to_pascal_case)
                        seg.replace("::", "_")
                            .split('_')
                            .map(|word| {
                                let mut chars = word.chars();
                                match chars.next() {
                                    None => String::new(),
                                    Some(first) => first.to_uppercase().chain(chars).collect(),
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .collect::<Vec<_>>()
                    .join("");

                // Add aliases for ALL names defined in this module's type code.
                // Strip the flat namespace prefix to produce leaf alias names.
                for mt in &sub.types {
                    for flat_name in Self::extract_defined_names(&mt.rendered) {
                        let leaf = if flat_name.starts_with(&flat_prefix) {
                            &flat_name[flat_prefix.len()..]
                        } else {
                            &flat_name
                        };
                        // Skip empty leaves (shouldn't happen, but be safe)
                        if !leaf.is_empty() {
                            writeln!(out, "{inner_indent}{leaf} = {flat_name}").unwrap();
                        }
                    }
                }

                // Recurse into submodules
                if !sub.submodules.is_empty() {
                    writeln!(out).unwrap();
                    sub.render_namespace_aliases_inner(out, indent_level + 1, &full_path);
                }

                writeln!(out).unwrap();
            }
        }
    }

    pub struct FileHeader {
        pub schema_name: String,
        pub schema_description: String,
    }

    impl FileHeader {
        pub fn render(&self) -> String {
            format!(
                "{}\nfrom __future__ import annotations\n",
                header_docstring(&self.schema_name, &self.schema_description),
            )
        }
    }

    /// Render the shared `"""..."""` docstring used at the top of every
    /// generated Python file. Mirrors the `// ...` block produced by the
    /// Rust and TypeScript generators so a reader sees the same wording
    /// regardless of which language they're looking at.
    pub fn header_docstring(schema_name: &str, schema_description: &str) -> String {
        let safe_name = super::sanitize_for_docstring(schema_name);
        let description_line = if schema_description.is_empty() {
            String::new()
        } else {
            format!("{}\n", super::sanitize_for_docstring(schema_description))
        };
        format!(
            "\"\"\"\n\
             DO NOT MODIFY THIS FILE MANUALLY\n\
             This file was generated by reflectapi-cli\n\
             \n\
             Schema name: {safe_name}\n\
             {description_line}\
             \"\"\"\n"
        )
    }

    pub struct Imports {
        pub has_async: bool,
        pub has_sync: bool,
        pub has_testing: bool,
        pub has_enums: bool,
        pub has_warnings: bool,
        pub extra_stdlib_imports: Vec<String>,
        pub extra_runtime_imports: Vec<String>,
        pub has_generics: bool,
        pub has_annotated: bool,
        pub has_literal: bool,
        pub has_discriminated_unions: bool,
        pub has_externally_tagged_enums: bool,
        pub global_type_vars: Vec<String>,
        /// At least one streaming endpoint is exposed by the client.
        pub has_streaming: bool,
        /// At least one generated class has a `reflectapi::Option<T>`
        /// field, so the bundle needs to import
        /// `ReflectapiPartialModel`.
        pub has_partial_models: bool,
    }

    pub struct DataClass {
        pub name: String,
        pub description: Option<String>,
        pub fields: Vec<Field>,
        pub is_tuple: bool,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
    }

    impl DataClass {
        /// True iff any field came from `reflectapi::Option<T>`; the
        /// containing class then needs `ReflectapiPartialModel` so that
        /// `model_fields_set` drives wire-format presence.
        fn is_partial(&self) -> bool {
            self.fields.iter().any(|f| f.is_partial)
        }

        pub fn render(&self) -> String {
            let mut s = String::new();
            let base_class = if self.is_partial() {
                "ReflectapiPartialModel"
            } else {
                "BaseModel"
            };
            if self.is_generic {
                let params = self.generic_params.join(", ");
                writeln!(s, "class {}({base_class}, Generic[{}]):", self.name, params).unwrap();
            } else {
                writeln!(s, "class {}({base_class}):", self.name).unwrap();
            }
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                }
            }
            writeln!(s).unwrap();
            // `validate_assignment=True` for partial models keeps
            // `model_fields_set` in sync when callers do
            // `m.field = value` after construction (so the assignment
            // actually lands on the wire). Plain BaseModel classes
            // don't need it.
            let config = if self.is_partial() {
                "ConfigDict(extra=\"ignore\", populate_by_name=True, validate_assignment=True)"
            } else {
                "ConfigDict(extra=\"ignore\", populate_by_name=True)"
            };
            writeln!(s, "    model_config = {config}").unwrap();
            writeln!(s).unwrap();
            for field in &self.fields {
                write!(s, "    {}: {}", field.name, field.type_annotation).unwrap();

                // Build Field() kwargs: description, alias, default
                let desc = field
                    .description
                    .as_ref()
                    .filter(|d| !d.is_empty() && !d.starts_with("(flattened"))
                    .map(|d| super::sanitize_for_string_literal(d));
                let has_field_args = desc.is_some()
                    || field.alias.is_some()
                    || (field.optional && field.alias.is_none());

                if has_field_args && (desc.is_some() || field.alias.is_some()) {
                    // Need Field() with named arguments
                    write!(s, " = Field(").unwrap();
                    let mut args = Vec::new();
                    if let Some(default) = &field.default_value {
                        args.push(format!("default={default}"));
                    } else if field.optional {
                        args.push("default=None".to_string());
                    }
                    if let Some(alias) = &field.alias {
                        args.push(format!("serialization_alias='{alias}'"));
                        args.push(format!("validation_alias='{alias}'"));
                    }
                    if let Some(ref d) = desc {
                        args.push(format!("description=\"{d}\""));
                    }
                    write!(s, "{}", args.join(", ")).unwrap();
                    write!(s, ")").unwrap();
                } else if field.optional {
                    write!(s, " = None").unwrap();
                } else if let Some(default) = &field.default_value {
                    write!(s, " = {default}").unwrap();
                }
                writeln!(s).unwrap();
            }
            writeln!(s).unwrap();
            s
        }
    }

    pub struct EnumClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<EnumVariant>,
    }

    impl EnumClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            writeln!(s, "class {}(str, Enum):", self.name).unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                }
            }
            writeln!(s).unwrap();
            if self.variants.is_empty() {
                writeln!(s, "    pass").unwrap();
            } else {
                for variant in &self.variants {
                    writeln!(s, "    {} = \"{}\"", variant.name, variant.value).unwrap();
                }
            }
            writeln!(s).unwrap();
            s
        }
    }

    pub struct PrimitiveEnumClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<PrimitiveEnumVariant>,
        pub is_int_enum: bool,
    }

    impl PrimitiveEnumClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            if self.is_int_enum {
                writeln!(s, "from enum import IntEnum").unwrap();
            } else {
                writeln!(s, "from enum import Enum").unwrap();
            }
            writeln!(s).unwrap();
            let base = if self.is_int_enum { "IntEnum" } else { "Enum" };
            writeln!(s, "class {}({}):", self.name, base).unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                writeln!(s).unwrap();
            }
            for variant in &self.variants {
                writeln!(s).unwrap();
                writeln!(s, "    {} = {}", variant.name, variant.value).unwrap();
                if let Some(desc) = &variant.description {
                    let desc = super::sanitize_for_docstring(desc);
                    writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                }
                writeln!(s).unwrap();
            }
            writeln!(s).unwrap();
            s
        }
    }

    pub struct UnionClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<UnionVariant>,
        pub discriminator_field: String,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
    }

    impl UnionClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            if self.is_generic {
                let params = self.generic_params.join(", ");
                writeln!(s).unwrap();
                writeln!(s, "class {}(Generic[{}]):", self.name, params).unwrap();
                let desc = super::sanitize_for_docstring(
                    self.description
                        .as_deref()
                        .unwrap_or("Generated discriminated union type."),
                );
                writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                writeln!(s).unwrap();
                writeln!(s, "    @classmethod").unwrap();
                writeln!(s, "    def __class_getitem__(cls, params):").unwrap();
                writeln!(
                    s,
                    "        \"\"\"Enable subscripting for generic discriminated union.\"\"\""
                )
                .unwrap();
                writeln!(s, "        if not isinstance(params, tuple):").unwrap();
                writeln!(s, "            params = (params,)").unwrap();
                writeln!(
                    s,
                    "        if len(params) != {}:",
                    self.generic_params.len()
                )
                .unwrap();
                writeln!(
                    s,
                    "            raise TypeError(f\"Expected {} type parameters, got {{len(params)}}\")",
                    self.generic_params.len()
                )
                .unwrap();
                writeln!(s).unwrap();
                // Build the Union expression
                let variant_exprs: Vec<String> = self
                    .variants
                    .iter()
                    .map(|v| {
                        let param_exprs: Vec<String> = (0..self.generic_params.len())
                            .map(|i| format!("params[{i}]"))
                            .collect();
                        format!("{}[{}]", v.base_name, param_exprs.join(", "))
                    })
                    .collect();
                writeln!(s, "        return Annotated[").unwrap();
                writeln!(s, "            Union[{}],", variant_exprs.join(", ")).unwrap();
                writeln!(
                    s,
                    "            Field(discriminator='{}')",
                    self.discriminator_field
                )
                .unwrap();
                writeln!(s, "        ]").unwrap();
            } else {
                writeln!(s).unwrap();
                writeln!(s, "class {}(RootModel):", self.name).unwrap();
                let type_annotations: Vec<&str> = self
                    .variants
                    .iter()
                    .map(|v| v.type_annotation.as_str())
                    .collect();
                writeln!(
                    s,
                    "    root: Annotated[Union[{}], Field(discriminator='{}')]",
                    type_annotations.join(", "),
                    self.discriminator_field
                )
                .unwrap();
            }
            writeln!(s).unwrap();
            if !self.is_generic {
                if let Some(desc) = &self.description {
                    let desc = super::sanitize_for_docstring(desc);
                    if !desc.is_empty() {
                        writeln!(s, "\"\"\"{desc}\"\"\"").unwrap();
                    }
                }
            }
            writeln!(s).unwrap();
            s
        }
    }

    pub struct UntaggedUnionClass {
        pub name: String,
        pub description: Option<String>,
        pub variants: Vec<UnionVariant>,
    }

    impl UntaggedUnionClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            let type_annotations: Vec<&str> = self
                .variants
                .iter()
                .map(|v| v.type_annotation.as_str())
                .collect();
            writeln!(s, "{} = Union[{}]", self.name, type_annotations.join(", ")).unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "\"\"\"{desc}\"\"\"").unwrap();
                }
            }
            writeln!(s).unwrap();
            s
        }
    }

    pub struct ClientClass {
        pub class_name: String,
        pub async_class_name: String,
        pub top_level_functions: Vec<Function>,
        pub function_groups: Vec<FunctionGroup>,
        pub generate_async: bool,
        pub generate_sync: bool,
        pub base_url: Option<String>,
    }

    impl ClientClass {
        pub fn render(&self) -> String {
            let mut s = String::new();

            if self.generate_async {
                // Async group classes
                for group in &self.function_groups {
                    writeln!(s).unwrap();
                    writeln!(s, "class Async{}:", group.class_name).unwrap();
                    writeln!(
                        s,
                        "    \"\"\"Async client for {} operations.\"\"\"",
                        group.name
                    )
                    .unwrap();
                    writeln!(s).unwrap();
                    writeln!(
                        s,
                        "    def __init__(self, client: AsyncClientBase) -> None:"
                    )
                    .unwrap();
                    writeln!(s, "        self._client = client").unwrap();

                    for function in &group.functions {
                        Self::write_function(&mut s, function, true, true, true);
                    }
                    writeln!(s).unwrap();
                }

                // Async client class
                writeln!(s).unwrap();
                writeln!(s, "class {}(AsyncClientBase):", self.async_class_name).unwrap();
                writeln!(s, "    \"\"\"Async client for the API.\"\"\"").unwrap();
                writeln!(s).unwrap();
                writeln!(s, "    def __init__(").unwrap();
                writeln!(s, "        self,").unwrap();
                if let Some(base_url) = &self.base_url {
                    writeln!(s, "        base_url: str = \"{base_url}\",").unwrap();
                } else {
                    writeln!(s, "        base_url: str,").unwrap();
                }
                writeln!(s, "        **kwargs: Any,").unwrap();
                writeln!(s, "    ) -> None:").unwrap();
                writeln!(s, "        super().__init__(base_url, **kwargs)").unwrap();
                writeln!(s).unwrap();
                for group in &self.function_groups {
                    writeln!(s).unwrap();
                    writeln!(
                        s,
                        "        self.{} = Async{}(self)",
                        group.name, group.class_name
                    )
                    .unwrap();
                }
                writeln!(s).unwrap();

                for function in &self.top_level_functions {
                    Self::write_function(&mut s, function, true, false, true);
                }
                writeln!(s).unwrap();
            }

            writeln!(s).unwrap();

            if self.generate_sync {
                // Sync group classes
                for group in &self.function_groups {
                    writeln!(s).unwrap();
                    writeln!(s, "class {}:", group.class_name).unwrap();
                    writeln!(
                        s,
                        "    \"\"\"Synchronous client for {} operations.\"\"\"",
                        group.name
                    )
                    .unwrap();
                    writeln!(s).unwrap();
                    writeln!(s, "    def __init__(self, client: ClientBase) -> None:").unwrap();
                    writeln!(s, "        self._client = client").unwrap();

                    for function in &group.functions {
                        Self::write_function(&mut s, function, false, true, true);
                    }
                    writeln!(s).unwrap();
                }

                // Sync client class
                writeln!(s).unwrap();
                writeln!(s, "class {}(ClientBase):", self.class_name).unwrap();
                writeln!(s, "    \"\"\"Synchronous client for the API.\"\"\"").unwrap();
                writeln!(s).unwrap();
                writeln!(s, "    def __init__(").unwrap();
                writeln!(s, "        self,").unwrap();
                if let Some(base_url) = &self.base_url {
                    writeln!(s, "        base_url: str = \"{base_url}\",").unwrap();
                } else {
                    writeln!(s, "        base_url: str,").unwrap();
                }
                writeln!(s, "        **kwargs: Any,").unwrap();
                writeln!(s, "    ) -> None:").unwrap();
                writeln!(s, "        super().__init__(base_url, **kwargs)").unwrap();
                writeln!(s).unwrap();
                for group in &self.function_groups {
                    writeln!(s).unwrap();
                    writeln!(
                        s,
                        "        self.{} = {}(self)",
                        group.name, group.class_name
                    )
                    .unwrap();
                }
                writeln!(s).unwrap();

                for function in &self.top_level_functions {
                    Self::write_function(&mut s, function, false, false, false);
                }
                writeln!(s).unwrap();
            }

            writeln!(s).unwrap();
            s
        }

        fn write_function(
            s: &mut String,
            function: &Function,
            is_async: bool,
            is_group: bool,
            use_raw_name_for_path_params: bool,
        ) {
            let is_stream = function.stream_item_type.is_some();
            writeln!(s).unwrap();
            // Method signature. Streaming methods are plain `def` regardless
            // of client kind: the runtime helper *is* the (async) generator,
            // so we just return it.
            if is_async && !is_stream {
                writeln!(s, "    async def {}(", function.name).unwrap();
            } else {
                writeln!(s, "    def {}(", function.name).unwrap();
            }
            writeln!(s, "        self,").unwrap();
            for param in &function.path_params {
                writeln!(s, "        {}: {},", param.name, param.type_annotation).unwrap();
            }
            if function.has_body {
                writeln!(s, "        data: Optional[{}] = None,", function.input_type).unwrap();
            }
            if let Some(headers_type) = &function.headers_type {
                writeln!(s, "        headers: Optional[{headers_type}] = None,").unwrap();
            }
            if let Some(item_type) = &function.stream_item_type {
                let iter_kind = if is_async {
                    "AsyncIterator"
                } else {
                    "Iterator"
                };
                writeln!(s, "    ) -> {iter_kind}[{item_type}]:").unwrap();
            } else if let Some(error_type) = &function.error_type {
                writeln!(
                    s,
                    "    ) -> ApiResponse[{}, {}]:",
                    function.output_type, error_type
                )
                .unwrap();
            } else {
                writeln!(s, "    ) -> ApiResponse[{}]:", function.output_type).unwrap();
            }

            // Docstring
            let desc = super::sanitize_for_docstring(function.description.as_deref().unwrap_or(""));
            write!(s, "        \"\"\"{desc}").unwrap();
            if function.has_body || !function.path_params.is_empty() {
                writeln!(s).unwrap();
                writeln!(s).unwrap();
                writeln!(s, "        Args:").unwrap();
                if function.has_body {
                    writeln!(
                        s,
                        "            data: Request data for the {} operation.",
                        function.name
                    )
                    .unwrap();
                }
                for param in &function.path_params {
                    let param_desc = super::sanitize_for_docstring(
                        param.description.as_deref().unwrap_or("Path parameter"),
                    );
                    writeln!(s, "            {}: {}", param.name, param_desc).unwrap();
                }
                writeln!(s).unwrap();
            } else {
                writeln!(s).unwrap();
                writeln!(s).unwrap();
            }
            writeln!(s, "        Returns:").unwrap();
            if let Some(item_type) = &function.stream_item_type {
                let iter_kind = if is_async {
                    "AsyncIterator"
                } else {
                    "Iterator"
                };
                writeln!(
                    s,
                    "            {iter_kind}[{item_type}]: SSE stream of {item_type} items"
                )
                .unwrap();
            } else if let Some(error_type) = &function.error_type {
                writeln!(
                    s,
                    "            ApiResponse[{}, {}]: Success={}, Error={}",
                    function.output_type, error_type, function.output_type, error_type
                )
                .unwrap();
            } else {
                writeln!(
                    s,
                    "            ApiResponse[{}]: Response containing {} data",
                    function.output_type, function.output_type
                )
                .unwrap();
            }
            if let Some(dep_note) = &function.deprecation_note {
                let dep_note = super::sanitize_for_docstring(dep_note);
                writeln!(s).unwrap();
                writeln!(s, "        .. deprecated::").unwrap();
                writeln!(s, "           {dep_note}").unwrap();
            }
            writeln!(s, "        \"\"\"").unwrap();

            // Deprecation warning
            if let Some(dep_note) = &function.deprecation_note {
                writeln!(s).unwrap();
                let warn_name = if is_group {
                    function.original_name.as_deref().unwrap_or(&function.name)
                } else {
                    &function.name
                };
                let warn_msg = if dep_note.is_empty() {
                    format!("{warn_name} is deprecated")
                } else {
                    format!("{warn_name} is deprecated: {dep_note}")
                };
                writeln!(s, "        warnings.warn(").unwrap();
                writeln!(s, "            \"{warn_msg}\",").unwrap();
                writeln!(s, "            DeprecationWarning,").unwrap();
                writeln!(s, "            stacklevel=2,").unwrap();
                writeln!(s, "        )").unwrap();
                writeln!(s).unwrap();
            }

            // Path
            writeln!(s, "        path = \"{}\"", function.path).unwrap();

            // Path parameters
            if !function.path_params.is_empty() {
                writeln!(
                    s,
                    "        # Format path parameters using safer string formatting"
                )
                .unwrap();
                writeln!(s, "        path_params = {{").unwrap();
                for param in &function.path_params {
                    let key = if use_raw_name_for_path_params {
                        &param.raw_name
                    } else {
                        &param.name
                    };
                    writeln!(s, "            \"{}\": str({}),", key, param.name).unwrap();
                }
                writeln!(s, "        }}").unwrap();
                writeln!(
                    s,
                    "        for param_name, param_value in path_params.items():"
                )
                .unwrap();
                writeln!(
                    s,
                    "            path = path.replace(\"{{\" + param_name + \"}}\", param_value)"
                )
                .unwrap();
            }
            writeln!(s).unwrap();

            // Request call
            writeln!(s, "        params: dict[str, Any] = {{}}").unwrap();

            let client_prefix = if is_group { "self._client." } else { "self." };
            if is_stream {
                // Streaming: just return the (async) generator from the
                // runtime helper — no await, no async-for indirection.
                writeln!(s, "        return {client_prefix}_make_sse_request(").unwrap();
            } else if is_async {
                writeln!(s, "        return await {client_prefix}_make_request(").unwrap();
            } else {
                writeln!(s, "        return {client_prefix}_make_request(").unwrap();
            }
            // HTTP method intentionally elided — every reflectapi endpoint
            // is POST by design, hardcoded in the runtime.
            writeln!(s, "            path,").unwrap();
            writeln!(s, "            params=params if params else None,").unwrap();
            if function.has_body {
                if function.is_input_primitive {
                    writeln!(s, "            json_data=data,").unwrap();
                } else {
                    writeln!(s, "            json_model=data,").unwrap();
                }
            }
            if function.headers_type.is_some() {
                writeln!(s, "            headers_model=headers,").unwrap();
            }
            if let Some(item_type) = &function.stream_item_type {
                writeln!(s, "            item_model={item_type},").unwrap();
            } else if function.output_type == "Any" {
                writeln!(s, "            response_model=None,").unwrap();
            } else {
                writeln!(s, "            response_model={},", function.output_type).unwrap();
            }
            if let Some(error_type) = &function.error_type {
                writeln!(s, "            error_model={error_type},").unwrap();
            }
            writeln!(s, "        )").unwrap();
            writeln!(s).unwrap();
        }
    }

    pub struct TestingModule {
        pub types: Vec<String>,
    }

    impl TestingModule {
        pub fn render(&self) -> String {
            let mut s = String::new();
            writeln!(s, "# Testing utilities").unwrap();
            writeln!(s).unwrap();
            for type_name in &self.types {
                // Convert dotted type ref to a valid Python function name fragment
                let func_suffix = type_name.replace('.', "_").to_lowercase();
                writeln!(s).unwrap();
                writeln!(
                    s,
                    "def create_{func_suffix}_response(value: {type_name}) -> ApiResponse[{type_name}]:",
                )
                .unwrap();
                writeln!(
                    s,
                    "    \"\"\"Create a mock ApiResponse for {type_name}.\"\"\""
                )
                .unwrap();
                writeln!(s, "    return create_api_response(value)").unwrap();
                writeln!(s).unwrap();
            }
            writeln!(s).unwrap();
            writeln!(s, "def create_mock_client() -> MockClient:").unwrap();
            writeln!(s, "    \"\"\"Create a mock client for testing.\"\"\"").unwrap();
            writeln!(s, "    return MockClient()").unwrap();
            s
        }
    }

    #[derive(Clone)]
    pub struct Field {
        pub name: String,
        pub type_annotation: String,
        pub description: Option<String>,
        pub deprecation_note: Option<String>,
        pub optional: bool,
        pub default_value: Option<String>,
        pub alias: Option<String>,
        /// `true` iff this field came from `reflectapi::Option<T>` in the
        /// source schema (the three-state Option). The renderer uses this
        /// to decide whether the containing class needs to inherit from
        /// `ReflectapiPartialModel`.
        pub is_partial: bool,
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
        /// SSE stream item type name; `None` for non-streaming endpoints.
        pub stream_item_type: Option<String>,
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
    pub struct UnitVariantClass {
        pub name: String,
        pub variant_name: String,
        pub description: Option<String>,
    }

    impl UnitVariantClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            writeln!(s, "class {}(BaseModel):", self.name).unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                } else {
                    writeln!(
                        s,
                        "    \"\"\"Unit variant for externally tagged enum.\"\"\""
                    )
                    .unwrap();
                }
            } else {
                writeln!(
                    s,
                    "    \"\"\"Unit variant for externally tagged enum.\"\"\""
                )
                .unwrap();
            }
            writeln!(s, "    model_config = ConfigDict(extra=\"ignore\")").unwrap();
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump(self, **kwargs):").unwrap();
            writeln!(
                s,
                "        \"\"\"Serialize as just the variant name string for unit variants.\"\"\""
            )
            .unwrap();
            writeln!(s, "        return \"{}\"", self.variant_name).unwrap();
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump_json(self, **kwargs):").unwrap();
            writeln!(
                s,
                "        \"\"\"Serialize as JSON string for unit variants.\"\"\""
            )
            .unwrap();
            writeln!(s, "        import json").unwrap();
            writeln!(s, "        return json.dumps(self.model_dump(**kwargs))").unwrap();
            s
        }
    }

    pub struct TupleVariantClass {
        pub name: String,
        pub variant_name: String,
        pub fields: Vec<Field>,
        pub description: Option<String>,
    }

    impl TupleVariantClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            writeln!(s, "class {}(BaseModel):", self.name).unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                } else {
                    writeln!(
                        s,
                        "    \"\"\"Tuple variant for externally tagged enum.\"\"\""
                    )
                    .unwrap();
                }
            } else {
                writeln!(
                    s,
                    "    \"\"\"Tuple variant for externally tagged enum.\"\"\""
                )
                .unwrap();
            }
            writeln!(s).unwrap();
            writeln!(s, "    model_config = ConfigDict(extra=\"ignore\")").unwrap();
            writeln!(s).unwrap();
            for field in &self.fields {
                write!(s, "    {}: {}", field.name, field.type_annotation).unwrap();
                if let Some(default) = &field.default_value {
                    write!(s, " = {default}").unwrap();
                } else if field.optional {
                    write!(s, " = None").unwrap();
                }
                writeln!(s).unwrap();
            }
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump(self, **kwargs):").unwrap();
            writeln!(
                s,
                "        \"\"\"Serialize as externally tagged tuple variant.\"\"\""
            )
            .unwrap();
            let field_refs: Vec<String> = self
                .fields
                .iter()
                .map(|f| format!("self.{}", f.name))
                .collect();
            writeln!(s, "        fields = [{}]", field_refs.join(", ")).unwrap();
            writeln!(s, "        return {{\"{}\": fields}}", self.variant_name).unwrap();
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump_json(self, **kwargs):").unwrap();
            writeln!(
                s,
                "        \"\"\"Serialize as JSON for externally tagged tuple variant.\"\"\""
            )
            .unwrap();
            writeln!(s, "        import json").unwrap();
            writeln!(s, "        return json.dumps(self.model_dump(**kwargs))").unwrap();
            s
        }
    }

    pub struct StructVariantClass {
        pub name: String,
        pub variant_name: String,
        pub fields: Vec<Field>,
        pub description: Option<String>,
    }

    impl StructVariantClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            writeln!(s, "class {}(BaseModel):", self.name).unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                } else {
                    writeln!(
                        s,
                        "    \"\"\"Struct variant for externally tagged enum.\"\"\""
                    )
                    .unwrap();
                }
            } else {
                writeln!(
                    s,
                    "    \"\"\"Struct variant for externally tagged enum.\"\"\""
                )
                .unwrap();
            }
            writeln!(s, "    model_config = ConfigDict(extra=\"ignore\")").unwrap();
            writeln!(s).unwrap();
            for field in &self.fields {
                write!(s, "    {}: {}", field.name, field.type_annotation).unwrap();
                if let Some(default) = &field.default_value {
                    write!(s, " = {default}").unwrap();
                } else if field.optional {
                    write!(s, " = None").unwrap();
                }
                writeln!(s).unwrap();
            }
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump(self, **kwargs):").unwrap();
            writeln!(
                s,
                "        \"\"\"Serialize as externally tagged struct variant.\"\"\""
            )
            .unwrap();
            writeln!(s, "        fields = {{}}").unwrap();
            for field in &self.fields {
                writeln!(
                    s,
                    "        if hasattr(self, '{}') and self.{} is not None:",
                    field.name, field.name
                )
                .unwrap();
                writeln!(
                    s,
                    "            fields['{}'] = self.{}",
                    field.name, field.name
                )
                .unwrap();
            }
            writeln!(s, "        return {{\"{}\": fields}}", self.variant_name).unwrap();
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump_json(self, **kwargs):").unwrap();
            writeln!(
                s,
                "        \"\"\"Serialize as JSON for externally tagged struct variant.\"\"\""
            )
            .unwrap();
            writeln!(s, "        import json").unwrap();
            writeln!(s, "        return json.dumps(self.model_dump(**kwargs))").unwrap();
            s
        }
    }

    pub struct ExternallyTaggedUnionClass {
        pub name: String,
        pub description: Option<String>,
        pub variant_definitions: Vec<String>,
        pub union_variant_names: Vec<String>,
    }

    impl ExternallyTaggedUnionClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            for variant_def in &self.variant_definitions {
                writeln!(s, "{variant_def}").unwrap();
                writeln!(s).unwrap();
            }
            writeln!(
                s,
                "{} = Union[{}]",
                self.name,
                self.union_variant_names.join(", ")
            )
            .unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "\"\"\"{desc}\"\"\"").unwrap();
                }
            }
            writeln!(s).unwrap();
            s
        }
    }

    pub struct TupleVariantDataClass {
        pub name: String,
        pub variant_name: String,
        pub field_types: Vec<String>,
        pub field_names: Vec<String>,
        pub description: Option<String>,
    }

    impl TupleVariantDataClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            writeln!(s, "@dataclass").unwrap();
            writeln!(s, "class {}:", self.name).unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                }
            }
            for field_type in &self.field_types {
                writeln!(s, "    {field_type}").unwrap();
            }
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump(self) -> dict:").unwrap();
            writeln!(
                s,
                "        \"\"\"Serialize tuple variant as externally tagged.\"\"\""
            )
            .unwrap();
            let field_refs: Vec<String> = self
                .field_names
                .iter()
                .map(|f| format!("self.{f}"))
                .collect();
            writeln!(s, "        fields = [{}]", field_refs.join(", ")).unwrap();
            writeln!(s, "        return {{\"{}\": fields}}", self.variant_name).unwrap();
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump_json(self, **kwargs) -> str:").unwrap();
            writeln!(s, "        \"\"\"Serialize as JSON.\"\"\"").unwrap();
            writeln!(s, "        import json").unwrap();
            writeln!(s, "        return json.dumps(self.model_dump())").unwrap();
            s
        }
    }

    pub struct StructVariantDataClass {
        pub name: String,
        pub variant_name: String,
        pub field_definitions: Vec<String>,
        pub field_names: Vec<String>,
        pub description: Option<String>,
    }

    impl StructVariantDataClass {
        pub fn render(&self) -> String {
            let mut s = String::new();
            writeln!(s, "@dataclass").unwrap();
            writeln!(s, "class {}:", self.name).unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                if !desc.is_empty() {
                    writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
                }
            }
            for field_def in &self.field_definitions {
                writeln!(s, "    {field_def}").unwrap();
            }
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump(self) -> dict:").unwrap();
            writeln!(
                s,
                "        \"\"\"Serialize struct variant as externally tagged.\"\"\""
            )
            .unwrap();
            writeln!(s, "        result = {{}}").unwrap();
            for field_name in &self.field_names {
                writeln!(
                    s,
                    "        if hasattr(self, '{field_name}') and self.{field_name} is not None:"
                )
                .unwrap();
                writeln!(s, "            result['{field_name}'] = self.{field_name}").unwrap();
            }
            writeln!(s, "        return {{\"{}\": result}}", self.variant_name).unwrap();
            writeln!(s).unwrap();
            writeln!(s, "    def model_dump_json(self, **kwargs) -> str:").unwrap();
            writeln!(s, "        \"\"\"Serialize as JSON.\"\"\"").unwrap();
            writeln!(s, "        import json").unwrap();
            writeln!(s, "        return json.dumps(self.model_dump())").unwrap();
            s
        }
    }

    pub struct DiscriminatedUnionEnum {
        pub variant_models: Vec<String>,
        pub union_members: Vec<String>,
        pub union_name: String,
        pub description: Option<String>,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
    }

    impl DiscriminatedUnionEnum {
        pub fn render(&self) -> String {
            let mut s = String::new();
            // TypeVar definitions are emitted once at the top of the file;
            // inline declarations are suppressed to avoid collisions with class names.
            for variant_model in &self.variant_models {
                writeln!(s, "{variant_model}").unwrap();
                writeln!(s).unwrap();
            }
            writeln!(s).unwrap();
            writeln!(s, "# Discriminated union for {}", self.union_name).unwrap();
            writeln!(
                s,
                "{} = Annotated[Union[{}], Field(discriminator='kind')]",
                self.union_name,
                self.union_members.join(", ")
            )
            .unwrap();
            if let Some(desc) = &self.description {
                let desc = super::sanitize_for_docstring(desc);
                writeln!(s, "\"\"\"{desc}\"\"\"").unwrap();
            }
            s
        }
    }

    pub struct ExternallyTaggedEnumCompact {
        pub name: String,
        pub description: Option<String>,
        pub variant_models: Vec<String>,
        pub union_variants: String,
        pub is_single_variant: bool,
        pub variant_entries: Vec<String>,
        pub serializer_entries: Vec<String>,
        /// Class names of non-unit variant types (for isinstance checks on direct instances)
        pub variant_class_names: Vec<String>,
        pub is_generic: bool,
        pub generic_params: Vec<String>,
    }

    impl ExternallyTaggedEnumCompact {
        pub fn render(&self) -> String {
            let mut s = String::new();
            for variant_model in &self.variant_models {
                writeln!(s, "{variant_model}").unwrap();
                writeln!(s).unwrap();
            }
            writeln!(s).unwrap();
            writeln!(s, "# Externally tagged enum using RootModel").unwrap();
            if self.is_single_variant {
                writeln!(s, "{}Variants = {}", self.name, self.union_variants).unwrap();
            } else {
                writeln!(s, "{}Variants = Union[{}]", self.name, self.union_variants).unwrap();
            }
            writeln!(s).unwrap();
            if self.is_generic {
                let params = self.generic_params.join(", ");
                writeln!(
                    s,
                    "class {}(RootModel[{}Variants], Generic[{}]):",
                    self.name, self.name, params
                )
                .unwrap();
            } else {
                writeln!(s, "class {}(RootModel[{}Variants]):", self.name, self.name).unwrap();
            }
            let desc = super::sanitize_for_docstring(
                self.description
                    .as_deref()
                    .unwrap_or("Externally tagged enum"),
            );
            writeln!(s, "    \"\"\"{desc}\"\"\"").unwrap();
            writeln!(s).unwrap();
            if self.is_generic {
                writeln!(s, "    @classmethod").unwrap();
                writeln!(s, "    def __class_getitem__(cls, params):").unwrap();
                writeln!(s, "        return cls").unwrap();
                writeln!(s).unwrap();
            }

            // Emit compact model_validator using helper
            writeln!(s, "    @model_validator(mode='before')").unwrap();
            writeln!(s, "    @classmethod").unwrap();
            writeln!(s, "    def _validate(cls, data):").unwrap();
            write!(s, "        return _parse_externally_tagged(data, {{").unwrap();
            for (i, entry) in self.variant_entries.iter().enumerate() {
                if i > 0 {
                    write!(s, ", ").unwrap();
                }
                write!(s, "{entry}").unwrap();
            }
            // Build the types tuple for isinstance checks on direct variant instances
            let types_tuple = if self.variant_class_names.is_empty() {
                "()".to_string()
            } else if self.variant_class_names.len() == 1 {
                format!("({},)", self.variant_class_names[0])
            } else {
                format!("({})", self.variant_class_names.join(", "))
            };
            writeln!(s, "}}, {types_tuple}, \"{name}\")", name = self.name).unwrap();
            writeln!(s).unwrap();

            // Emit compact model_serializer using helper
            writeln!(s, "    @model_serializer").unwrap();
            writeln!(s, "    def _serialize(self):").unwrap();
            write!(
                s,
                "        return _serialize_externally_tagged(self.root, {{"
            )
            .unwrap();
            for (i, entry) in self.serializer_entries.iter().enumerate() {
                if i > 0 {
                    write!(s, ", ").unwrap();
                }
                write!(s, "{entry}").unwrap();
            }
            writeln!(s, "}}, \"{name}\")", name = self.name).unwrap();
            s
        }
    }
}

/// Pydantic equivalent of monomorphization: for any struct that uses
/// `serde(flatten)` over a generic parameter, rewrite every concrete
/// instantiation as its own non-generic struct in the schema. The
/// existing flatten-of-concrete-type rendering then handles them
/// correctly.
///
/// Rust and TypeScript handle this case dynamically (serde flatten at
/// runtime; intersection types at the type level). Pydantic has no
/// equivalent — `Generic[T]` can't be told to lift T's fields into the
/// outer model when T is bound. So we resolve the polymorphism here.
fn monomorphize_flatten_generics(schema: &mut Schema) -> anyhow::Result<()> {
    use reflectapi_schema::{Instantiate as _, Visitor};
    use std::ops::ControlFlow;

    // 1. Mark structs whose fields flatten a generic parameter
    //    (directly, or wrapped in Option<T>).
    fn flattened_target_is_typevar(type_ref: &TypeReference, params: &BTreeSet<&str>) -> bool {
        if params.contains(type_ref.name.as_str()) {
            return true;
        }
        // serde unwraps Option in flatten position, so a flattened
        // `Option<T>` still requires monomorphization.
        if (type_ref.name == "std::option::Option" || type_ref.name == "reflectapi::Option")
            && !type_ref.arguments.is_empty()
        {
            return flattened_target_is_typevar(&type_ref.arguments[0], params);
        }
        false
    }

    let mut marked: BTreeSet<String> = {
        let mut out = BTreeSet::new();
        for ts in [&schema.input_types, &schema.output_types] {
            for typ in ts.types() {
                if let Type::Struct(s) = typ {
                    if s.parameters.is_empty() {
                        continue;
                    }
                    let params: BTreeSet<&str> =
                        s.parameters.iter().map(|p| p.name.as_str()).collect();
                    if s.fields
                        .iter()
                        .any(|f| f.flattened() && flattened_target_is_typevar(&f.type_ref, &params))
                    {
                        out.insert(s.name.clone());
                    }
                }
            }
        }
        out
    };
    if marked.is_empty() {
        return Ok(());
    }

    // Transitive marking: a generic struct G that doesn't itself have
    // flatten-over-typevar must still be monomorphized if any of its
    // fields references a (transitively-)marked struct using G's
    // TypeVars as args. Otherwise, when concrete usages of G are
    // resolved to monomorphized forms, the inner marked-struct ref
    // would be left dangling at G's TypeVar — and removing the
    // original marked struct would leave G's leftover reference
    // pointing at nothing.
    //
    // Iterate to fixed point so chains like A<T> → B<T> → MarkedC<T>
    // all get marked.
    /// True if `tr`'s tree mentions any of the enclosing TypeVars at
    /// any depth. A TypeVar leaf is a zero-arg ref whose name is one
    /// of the enclosing parameters.
    fn ref_tree_contains_typevar(tr: &TypeReference, typevars: &BTreeSet<&str>) -> bool {
        if tr.arguments.is_empty() && typevars.contains(tr.name.as_str()) {
            return true;
        }
        tr.arguments
            .iter()
            .any(|a| ref_tree_contains_typevar(a, typevars))
    }
    fn ref_uses_marked_with_typevar(
        type_ref: &TypeReference,
        marked: &BTreeSet<String>,
        typevars: &BTreeSet<&str>,
    ) -> bool {
        // Marked struct used with at least one arg that mentions an
        // enclosing TypeVar somewhere in its tree (not just the
        // immediate arg). `Marked<Option<I>>`, `Marked<Vec<Box<T>>>`,
        // and `Marked<I>` all qualify — without this, the wrapper
        // doesn't get transitively marked, the inner marked struct
        // gets removed in step 6, and the wrapper's ref dangles.
        if marked.contains(&type_ref.name)
            && type_ref
                .arguments
                .iter()
                .any(|a| ref_tree_contains_typevar(a, typevars))
        {
            return true;
        }
        type_ref
            .arguments
            .iter()
            .any(|a| ref_uses_marked_with_typevar(a, marked, typevars))
    }
    loop {
        let mut new_marks: BTreeSet<String> = BTreeSet::new();
        for ts in [&schema.input_types, &schema.output_types] {
            for typ in ts.types() {
                match typ {
                    Type::Struct(s) => {
                        if s.parameters.is_empty() || marked.contains(&s.name) {
                            continue;
                        }
                        let typevars: BTreeSet<&str> =
                            s.parameters.iter().map(|p| p.name.as_str()).collect();
                        if s.fields
                            .iter()
                            .any(|f| ref_uses_marked_with_typevar(&f.type_ref, &marked, &typevars))
                        {
                            new_marks.insert(s.name.clone());
                        }
                    }
                    Type::Enum(e) => {
                        if e.parameters.is_empty() || marked.contains(&e.name) {
                            continue;
                        }
                        let typevars: BTreeSet<&str> =
                            e.parameters.iter().map(|p| p.name.as_str()).collect();
                        let any = e.variants.iter().any(|v| {
                            v.fields.iter().any(|f| {
                                ref_uses_marked_with_typevar(&f.type_ref, &marked, &typevars)
                            })
                        });
                        if any {
                            new_marks.insert(e.name.clone());
                        }
                    }
                    Type::Primitive(_) => {}
                }
            }
        }
        if new_marks.is_empty() {
            break;
        }
        marked.extend(new_marks);
    }

    // 2. Walk every type reference reachable from functions and types,
    //    rewriting marked refs in place (bottom-up) to their mangled
    //    monomorphized names. Each new (struct, normalized_args) pair
    //    is registered in `concrete`. Nested patterns like
    //    `OuterMarked<InnerMarked<I, D>, C>` resolve correctly because
    //    the inner ref is normalized first, so the outer's key is
    //    `(OuterMarked, [InnerMarked_I_D, C])` — concrete on both
    //    sides.
    let mut concrete: BTreeMap<(String, Vec<TypeReference>), String> = BTreeMap::new();
    let mut registered: BTreeSet<String> = BTreeSet::new();

    /// A type-ref is "concrete" if it resolves to a real schema type,
    /// to one of our own monomorphized names (which will be inserted
    /// in step 4), or is built up from concrete refs. A bare ref like
    /// `T` with no arguments, no schema entry, and not yet registered
    /// as a mono'd name is a TypeVar from some enclosing generic
    /// context — substituting it now would produce a struct body
    /// that still references a TypeVar.
    fn is_concrete(
        type_ref: &TypeReference,
        schema: &Schema,
        registered: &BTreeSet<String>,
    ) -> bool {
        if type_ref.arguments.is_empty()
            && schema.get_type(&type_ref.name).is_none()
            && !registered.contains(&type_ref.name)
        {
            return false;
        }
        type_ref
            .arguments
            .iter()
            .all(|a| is_concrete(a, schema, registered))
    }

    fn normalize_marked_refs(
        type_ref: &mut TypeReference,
        marked: &BTreeSet<String>,
        concrete: &mut BTreeMap<(String, Vec<TypeReference>), String>,
        registered: &mut BTreeSet<String>,
        schema: &Schema,
    ) -> anyhow::Result<()> {
        // Bottom-up: arguments first, so this ref's args are already
        // resolved by the time we form the lookup key.
        for a in type_ref.arguments.iter_mut() {
            normalize_marked_refs(a, marked, concrete, registered, schema)?;
        }
        if !marked.contains(&type_ref.name) || type_ref.arguments.is_empty() {
            return Ok(());
        }
        // If any arg is a TypeVar from an enclosing generic context
        // (e.g. `Inner<T>` inside `struct Outer<T> { ... }`), this
        // ref isn't a concrete instantiation — it's a generic-position
        // use of a marked struct. Don't register: when the enclosing
        // generic resolves to a concrete instantiation, the
        // substituted args will be concrete and we'll register then.
        if !type_ref
            .arguments
            .iter()
            .all(|a| is_concrete(a, schema, registered))
        {
            return Ok(());
        }
        let key = (type_ref.name.clone(), type_ref.arguments.clone());
        let mangled = if let Some(existing) = concrete.get(&key) {
            existing.clone()
        } else {
            let typ = schema
                .input_types
                .get_type(&type_ref.name)
                .or_else(|| schema.output_types.get_type(&type_ref.name))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "monomorphization: marked type '{}' missing from schema",
                        type_ref.name
                    )
                })?;
            // Register before recursing so cycles can't loop forever
            // (a marked type that transitively contains a ref to itself
            // with the same args would otherwise spin).
            let mangled = mangle_monomorphized_name(&type_ref.name, &type_ref.arguments);
            concrete.insert(key, mangled.clone());
            registered.insert(mangled.clone());

            // Substitute with the (already-normalized) args and recurse
            // into the resulting variants/fields so any further marked
            // refs there get registered too. Both Struct and Enum
            // implement Instantiate.
            match typ {
                Type::Struct(s) => {
                    let s = s.clone();
                    if s.parameters.len() != type_ref.arguments.len() {
                        anyhow::bail!(
                            "monomorphization: arity mismatch for struct '{}': expected {}, got {}",
                            type_ref.name,
                            s.parameters.len(),
                            type_ref.arguments.len(),
                        );
                    }
                    let mono = s.instantiate(&type_ref.arguments);
                    for f in mono.fields.iter() {
                        let mut tr = f.type_ref.clone();
                        normalize_marked_refs(&mut tr, marked, concrete, registered, schema)?;
                    }
                }
                Type::Enum(e) => {
                    let e = e.clone();
                    if e.parameters.len() != type_ref.arguments.len() {
                        anyhow::bail!(
                            "monomorphization: arity mismatch for enum '{}': expected {}, got {}",
                            type_ref.name,
                            e.parameters.len(),
                            type_ref.arguments.len(),
                        );
                    }
                    let mono = e.instantiate(&type_ref.arguments);
                    for v in mono.variants.iter() {
                        for f in v.fields.iter() {
                            let mut tr = f.type_ref.clone();
                            normalize_marked_refs(&mut tr, marked, concrete, registered, schema)?;
                        }
                    }
                }
                Type::Primitive(_) => {
                    anyhow::bail!(
                        "monomorphization: marked '{}' resolved to a primitive — only structs and enums can be monomorphized",
                        type_ref.name,
                    );
                }
            }
            mangled
        };
        type_ref.name = mangled;
        type_ref.arguments.clear();
        Ok(())
    }

    // We mutate clones to drive registration; the schema itself is
    // rewritten in step 5 by the Visitor. Doing it here too would
    // double-edit refs we've already normalized.
    let mut seeds: Vec<TypeReference> = Vec::new();
    for f in &schema.functions {
        if let Some(t) = &f.input_type {
            seeds.push(t.clone());
        }
        if let Some(t) = &f.input_headers {
            seeds.push(t.clone());
        }
        match &f.output_type {
            OutputType::Complete { output_type } => {
                if let Some(t) = output_type {
                    seeds.push(t.clone());
                }
            }
            OutputType::Stream { item_type } => {
                seeds.push(item_type.clone());
            }
        }
        if let Some(t) = &f.error_type {
            seeds.push(t.clone());
        }
    }
    for ts in [&schema.input_types, &schema.output_types] {
        for typ in ts.types() {
            match typ {
                Type::Struct(s) => {
                    for field in s.fields.iter() {
                        seeds.push(field.type_ref.clone());
                    }
                }
                Type::Enum(e) => {
                    for v in &e.variants {
                        for field in v.fields.iter() {
                            seeds.push(field.type_ref.clone());
                        }
                    }
                }
                Type::Primitive(_) => {}
            }
        }
    }
    for mut seed in seeds {
        normalize_marked_refs(&mut seed, &marked, &mut concrete, &mut registered, schema)?;
    }

    // 4a. Sanity-check the mangling table for collisions before
    //     inserting. If two distinct (struct, args) keys resolved to
    //     the same mangled name, `insert_type` would silently drop
    //     the second insert and one schema's body would survive
    //     under both keys' lookups — exactly the silent-data-loss
    //     class of bug we're trying to prevent.
    {
        let mut seen: BTreeMap<&String, &(String, Vec<TypeReference>)> = BTreeMap::new();
        for (key, mangled) in concrete.iter() {
            if let Some(prev) = seen.insert(mangled, key) {
                anyhow::bail!(
                    "monomorphization: mangled-name collision: \
                     '{mangled}' generated by both {key:?} and {prev:?}. \
                     This is a codegen bug — please file an issue with \
                     the offending schema.",
                );
            }
        }
    }

    // 4. Insert monomorphized types into both typespaces (mirroring the
    //    location of the original generic). Both struct and enum
    //    instantiations are supported — a generic enum used as a
    //    variant container for marked structs needs its own concrete
    //    monomorphizations or the variant's field references would
    //    dangle after step 6 removes the originals.
    for ((name, args), mangled) in concrete.iter() {
        for ts_is_input in [true, false] {
            let ts = if ts_is_input {
                &schema.input_types
            } else {
                &schema.output_types
            };
            let typ = match ts.get_type(name) {
                Some(t) => t.clone(),
                None => continue,
            };
            let mono = match typ {
                Type::Struct(s) => {
                    let mut mono = s.instantiate(args);
                    mono.name = mangled.clone();
                    Type::Struct(mono)
                }
                Type::Enum(e) => {
                    let mut mono = e.instantiate(args);
                    mono.name = mangled.clone();
                    Type::Enum(mono)
                }
                Type::Primitive(_) => continue,
            };
            let target = if ts_is_input {
                &mut schema.input_types
            } else {
                &mut schema.output_types
            };
            target.insert_type(mono);
        }
    }

    // 5. Rewrite every type reference of the form `Marked<args>` to the
    //    mangled name with no args.
    struct Rewriter<'a> {
        table: &'a BTreeMap<(String, Vec<TypeReference>), String>,
    }
    impl<'a> Visitor for Rewriter<'a> {
        type Output = ();
        fn visit_type_ref(
            &mut self,
            type_ref: &mut TypeReference,
        ) -> ControlFlow<Self::Output, Self::Output> {
            // Recurse into args first so inner refs resolve before outer
            // lookup. Cleared args after the outer match means leaf-first
            // is fine here too.
            for a in type_ref.arguments.iter_mut() {
                let _ = self.visit_type_ref(a);
            }
            let key = (type_ref.name.clone(), type_ref.arguments.clone());
            if let Some(mangled) = self.table.get(&key) {
                type_ref.name = mangled.clone();
                type_ref.arguments.clear();
            }
            ControlFlow::Continue(())
        }
    }
    let mut r = Rewriter { table: &concrete };
    let _ = r.visit_schema_inputs(schema);
    let _ = r.visit_schema_outputs(schema);

    // 6. Drop the original generic marked types — they have no live
    //    references after rewriting, and emitting them would produce a
    //    bare `Generic[T]` class with the flatten field missing (the
    //    bug we're fixing). `remove_type` invalidates the typespace's
    //    internal index map, so subsequent lookups rebuild it.
    for name in &marked {
        let _ = schema.input_types.remove_type(name);
        let _ = schema.output_types.remove_type(name);
    }

    // 7. Post-pass invariant check (debug builds). Catches two
    //    failure classes:
    //      - dangling refs to a removed marked type (anywhere)
    //      - TypeVar leak in a body we *just monomorphized* (only
    //        on types we created — original schema bodies can
    //        legitimately reference external names like
    //        chrono::Utc as type-arg phantoms or const-generic
    //        scalars that aren't standalone schema entries)
    //
    //    Run in debug builds only. Errors here mean the pass has
    //    a logic bug; that's a developer-visible problem.
    debug_assert_monomorphization_invariants(schema, &marked, &registered);

    Ok(())
}

#[cfg(debug_assertions)]
// `monomorphized` holds the names of types we just created (the
// values of the `concrete` table). Only those bodies need the
// TypeVar-leak check — everything else was in the schema before
// this pass and isn't our concern here.
fn debug_assert_monomorphization_invariants(
    schema: &Schema,
    marked: &BTreeSet<String>,
    monomorphized: &BTreeSet<String>,
) {
    // Walk every type ref reachable from functions and types.
    fn walk_ref<F: FnMut(&TypeReference)>(tr: &TypeReference, f: &mut F) {
        f(tr);
        for a in &tr.arguments {
            walk_ref(a, f);
        }
    }

    let mut violations: Vec<String> = Vec::new();

    // Check 1: no dangling refs to removed marked types, anywhere
    // they could appear. Function-level refs first.
    for fn_def in &schema.functions {
        for site in [
            fn_def.input_type.as_ref(),
            fn_def.input_headers.as_ref(),
            match &fn_def.output_type {
                OutputType::Complete { output_type } => output_type.as_ref(),
                OutputType::Stream { item_type } => Some(item_type),
            },
            fn_def.error_type.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            walk_ref(site, &mut |tr| {
                if marked.contains(&tr.name) {
                    violations.push(format!(
                        "function {:?} still references removed marked type '{}' \
                         (post-pass dangling ref)",
                        fn_def.name, tr.name,
                    ));
                }
            });
        }
    }

    // Then in every type's body. Plus a narrower TypeVar-leak
    // check that only fires inside bodies of types we just
    // monomorphized.
    for ts in [&schema.input_types, &schema.output_types] {
        for typ in ts.types() {
            let (type_name, fields_iter): (&str, Box<dyn Iterator<Item = (&str, &TypeReference)>>) =
                match typ {
                    Type::Struct(s) => (
                        s.name.as_str(),
                        Box::new(s.fields.iter().map(|f| (f.name(), &f.type_ref))),
                    ),
                    Type::Enum(e) => (
                        e.name.as_str(),
                        Box::new(
                            e.variants
                                .iter()
                                .flat_map(|v| v.fields.iter().map(|f| (f.name(), &f.type_ref))),
                        ),
                    ),
                    Type::Primitive(_) => continue,
                };
            let we_monomorphized_this = monomorphized.contains(type_name);
            for (field_name, type_ref) in fields_iter {
                walk_ref(type_ref, &mut |tr| {
                    if marked.contains(&tr.name) {
                        violations.push(format!(
                            "{type_name:?}, field {field_name:?} references removed marked type '{}'",
                            tr.name,
                        ));
                    }
                    if we_monomorphized_this
                        && tr.arguments.is_empty()
                        && schema.get_type(&tr.name).is_none()
                    {
                        violations.push(format!(
                            "{type_name:?}, field {field_name:?} references unknown type '{}' \
                             (TypeVar leak in monomorphized body)",
                            tr.name,
                        ));
                    }
                });
            }
        }
    }
    if !violations.is_empty() {
        panic!(
            "monomorphize_flatten_generics: post-pass invariants violated:\n  {}",
            violations.join("\n  "),
        );
    }
}

#[cfg(not(debug_assertions))]
#[allow(dead_code)]
fn debug_assert_monomorphization_invariants(
    _schema: &Schema,
    _marked: &BTreeSet<String>,
    _monomorphized: &BTreeSet<String>,
) {
}

/// Mangle a `(struct, args)` instantiation into a fresh schema type
/// name. The struct's full namespace-qualified name is preserved so
/// downstream namespace mangling stays sensible; the suffix encodes
/// each argument's *full* path (`::` flattened to `_`) so two args
/// that share a leaf name but live in different modules don't fuse
/// into one mangled name (which would silently lose one type's
/// fields).
///
/// If the resulting suffix is long enough that downstream class-name
/// handling would hash-truncate (and produce a mismatch between the
/// truncated class definition and the un-truncated dotted-path
/// reference), hash the suffix here. Same hash on both sides → all
/// references stay consistent.
fn mangle_monomorphized_name(struct_name: &str, args: &[TypeReference]) -> String {
    fn arg_suffix(arg: &TypeReference) -> String {
        let base = arg.name.replace("::", "_");
        if arg.arguments.is_empty() {
            base
        } else {
            let inner = arg
                .arguments
                .iter()
                .map(arg_suffix)
                .collect::<Vec<_>>()
                .join("_");
            format!("{base}_{inner}")
        }
    }
    let suffix = args.iter().map(arg_suffix).collect::<Vec<_>>().join("_");

    // Length budget: the FLAT PascalCase class name (namespace +
    // struct + suffix, with `_` collapsed) must stay under 80 chars
    // or downstream `finalize_class_name` will hash-truncate, while
    // `type_name_to_python_ref` would still emit the un-truncated
    // form — yielding two inconsistent names. Predict the
    // post-PascalCase length from the struct's original name + the
    // suffix, hash if it would overflow.
    let candidate = format!("{struct_name}_{suffix}");
    let pascal_len = pascal_case_len(&candidate);
    const FLAT_NAME_BUDGET: usize = 80;
    if pascal_len <= FLAT_NAME_BUDGET {
        return candidate;
    }
    // Trim the suffix to fit. We keep as much human-readable prefix as
    // we can; the remaining bytes are a stable 8-hex-char hash.
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    struct_name.hash(&mut h);
    suffix.hash(&mut h);
    let hash = format!("{:08x}", h.finish() & 0xFFFF_FFFF);
    // Pascal-collapsed budget for the suffix:
    let head_pascal = pascal_case_len(&format!("{struct_name}_"));
    let hash_pascal = pascal_case_len(&format!("_{hash}"));
    let suffix_budget = FLAT_NAME_BUDGET.saturating_sub(head_pascal + hash_pascal);
    // Walk the suffix one byte at a time tracking PascalCase length.
    let mut keep = 0usize;
    let mut pascal = 0usize;
    let bytes = suffix.as_bytes();
    while keep < bytes.len() && pascal < suffix_budget {
        if bytes[keep] != b'_' {
            pascal += 1;
        }
        keep += 1;
    }
    let trimmed = std::str::from_utf8(&bytes[..keep]).unwrap_or("");
    format!("{struct_name}_{trimmed}_{hash}")
}

/// Count the chars that a `_`-separated identifier would have once
/// PascalCased (to_pascal_case) — `_` becomes a word boundary that
/// disappears, so the post-Pascal length is the number of non-`_`
/// chars.
fn pascal_case_len(s: &str) -> usize {
    s.bytes().filter(|&b| b != b'_' && b != b':').count()
}

#[cfg(test)]
mod tests {
    use super::{build_python_class_name_map, generate_init_py, mangle_monomorphized_name, Config};
    use crate::TypeReference;

    #[test]
    fn python_init_exports_client() {
        let init_py = generate_init_py(&Config::default());
        assert!(init_py.contains("from ._client import AsyncClient, Client"));
        assert!(init_py.contains("__all__ = [\"AsyncClient\", \"Client\"]"));
        assert!(!init_py.contains("SyncClient"));
    }

    #[test]
    fn destutter_falls_back_on_collision() {
        let class_names = build_python_class_name_map([
            "OfferRequestPartIdentity",
            "offer_request::OfferRequestPartIdentity",
            "system::SystemVersionInfo",
        ]);

        assert_eq!(
            class_names.get("OfferRequestPartIdentity").unwrap(),
            "OfferRequestPartIdentity"
        );
        assert_eq!(
            class_names
                .get("offer_request::OfferRequestPartIdentity")
                .unwrap(),
            "OfferRequestOfferRequestPartIdentity"
        );
        assert_eq!(
            class_names.get("system::SystemVersionInfo").unwrap(),
            "SystemVersionInfo"
        );
    }

    fn tref(name: &str) -> TypeReference {
        TypeReference {
            name: name.to_string(),
            arguments: vec![],
        }
    }

    fn tref_args(name: &str, args: Vec<TypeReference>) -> TypeReference {
        TypeReference {
            name: name.to_string(),
            arguments: args,
        }
    }

    #[test]
    fn mangler_distinguishes_namespaces_with_same_leaf() {
        // The pre-fix mangler used only the leaf, so a::Sample and
        // b::Sample collided and one struct's fields were silently
        // dropped (cf. test_generic_flatten_leaf_collision).
        let m1 = mangle_monomorphized_name("Wrap", &[tref("a::Sample")]);
        let m2 = mangle_monomorphized_name("Wrap", &[tref("b::Sample")]);
        assert_ne!(
            m1, m2,
            "same-leaf args from distinct namespaces must mangle distinctly"
        );
    }

    #[test]
    fn mangler_distinguishes_arity_and_order() {
        let foo_bar = mangle_monomorphized_name("Wrap", &[tref("Foo"), tref("Bar")]);
        let bar_foo = mangle_monomorphized_name("Wrap", &[tref("Bar"), tref("Foo")]);
        let foo_only = mangle_monomorphized_name("Wrap", &[tref("Foo")]);
        assert_ne!(foo_bar, bar_foo, "arg order must affect mangling");
        assert_ne!(foo_bar, foo_only, "arity must affect mangling");
    }

    #[test]
    fn mangler_distinguishes_nested_args() {
        // Foo<Vec<Bar>> vs Foo<Bar> — the latter is a single concrete
        // arg, the former wraps. The mangler must not collapse them.
        let plain = mangle_monomorphized_name("Wrap", &[tref("Bar")]);
        let nested =
            mangle_monomorphized_name("Wrap", &[tref_args("std::vec::Vec", vec![tref("Bar")])]);
        assert_ne!(plain, nested);
    }

    #[test]
    fn mangler_long_input_stays_under_class_name_budget() {
        // Realistic deep-nesting case. The post-PascalCase length
        // must not exceed 80 — otherwise downstream
        // `finalize_class_name` would hash-truncate while
        // `type_name_to_python_ref` would emit the un-truncated form
        // and the two refs would disagree.
        let deep = mangle_monomorphized_name(
            "very::long::module::path::OuterWrapper",
            &[
                tref_args(
                    "another::very::long::module::path::InnerWrapper",
                    vec![tref("nested::module::ConcreteTypeName")],
                ),
                tref("yet::another::module::path::SecondArg"),
            ],
        );
        let pascal_len = super::pascal_case_len(&deep);
        assert!(
            pascal_len <= 80,
            "post-Pascal mangled name '{deep}' exceeds 80 chars (got {pascal_len})",
        );
    }

    /// Regression: the post-pass invariant check used to flag
    /// chrono::Utc as a "TypeVar leak" because it appears as an arg
    /// of chrono::DateTime<Tz> but isn't a standalone schema type.
    /// Const-generic scalars (`[T; 2]`'s 2) had the same issue.
    /// Both are external/structural names that legitimately don't
    /// have schema entries — the check should only fire on bodies
    /// of types we actually monomorphized.
    #[test]
    fn invariant_check_ignores_phantom_type_args_in_unmodified_bodies() {
        // Marked struct + a struct holding a chrono::DateTime<Utc>
        // field. After monomorphization, the marked struct goes
        // away and the holder remains, with its DateTime<Utc> field
        // intact. `chrono::Utc` is registered nowhere — the
        // earlier validator would panic; the narrowed one must not.
        let schema_json = r#"{
            "name": "test",
            "functions": [{
                "name": "f", "path": "",
                "input_type": { "name": "Holder" },
                "output_kind": "complete",
                "output_type": { "name": "Wrapper", "arguments": [{ "name": "Holder" }] },
                "serialization": ["json"]
            }],
            "input_types": { "types": [
                { "kind": "primitive", "name": "chrono::DateTime", "description": "ts",
                  "parameters": [{ "name": "Tz" }],
                  "fallback": { "name": "std::string::String" } },
                { "kind": "primitive", "name": "std::string::String", "description": "s" },
                { "kind": "struct", "name": "Holder",
                  "fields": { "named": [
                    { "name": "ts",
                      "type": { "name": "chrono::DateTime",
                                "arguments": [{ "name": "chrono::Utc" }] },
                      "required": true }
                  ] } },
                { "kind": "struct", "name": "Wrapper",
                  "parameters": [{ "name": "T" }],
                  "fields": { "named": [
                    { "name": "inner", "type": { "name": "T" }, "required": true, "flattened": true }
                  ] } }
            ] },
            "output_types": { "types": [
                { "kind": "primitive", "name": "chrono::DateTime", "description": "ts",
                  "parameters": [{ "name": "Tz" }],
                  "fallback": { "name": "std::string::String" } },
                { "kind": "primitive", "name": "std::string::String", "description": "s" },
                { "kind": "struct", "name": "Holder",
                  "fields": { "named": [
                    { "name": "ts",
                      "type": { "name": "chrono::DateTime",
                                "arguments": [{ "name": "chrono::Utc" }] },
                      "required": true }
                  ] } },
                { "kind": "struct", "name": "Wrapper",
                  "parameters": [{ "name": "T" }],
                  "fields": { "named": [
                    { "name": "inner", "type": { "name": "T" }, "required": true, "flattened": true }
                  ] } }
            ] }
        }"#;
        let schema: crate::Schema = serde_json::from_str(schema_json).unwrap();
        // No panic: the validator must not flag chrono::Utc.
        let py = super::generate(schema, &Config::default()).expect("codegen must run cleanly");
        // Sanity: the chrono mapping made it to the output.
        assert!(py.contains("datetime") || py.contains("Holder"));
    }

    /// End-to-end: a marked struct that flattens its generic param
    /// AND references itself recursively (`Vec<Self<T>>`). The
    /// reflectapi Rust derive macros can't construct such a type
    /// (they overflow during schema building), but a hand-built or
    /// JSON-loaded schema can — and the codegen must terminate
    /// thanks to the register-before-recurse guard in
    /// `normalize_marked_refs`. Without that guard, the second
    /// recursive lookup would re-register and spin.
    #[test]
    fn recursive_marked_struct_terminates_and_renders() {
        let schema_json = r#"{
            "name": "test",
            "functions": [{
                "name": "f",
                "path": "",
                "input_type": { "name": "FlatTree", "arguments": [{ "name": "Item" }] },
                "output_kind": "complete",
                "output_type": { "name": "FlatTree", "arguments": [{ "name": "Item" }] },
                "serialization": ["json"]
            }],
            "input_types": { "types": [
                { "kind": "primitive", "name": "std::string::String", "description": "UTF-8" },
                { "kind": "primitive", "name": "u32", "description": "u32" },
                { "kind": "primitive", "name": "std::vec::Vec", "description": "Vec",
                  "parameters": [{ "name": "T" }] },
                { "kind": "struct", "name": "Item",
                  "fields": { "named": [
                    { "name": "id", "type": { "name": "u32" }, "required": true },
                    { "name": "label", "type": { "name": "std::string::String" }, "required": true }
                  ] } },
                { "kind": "struct", "name": "FlatTree",
                  "parameters": [{ "name": "T" }],
                  "fields": { "named": [
                    { "name": "value", "type": { "name": "T" }, "required": true, "flattened": true },
                    { "name": "children",
                      "type": { "name": "std::vec::Vec",
                                "arguments": [{ "name": "FlatTree", "arguments": [{ "name": "T" }] }] },
                      "required": true }
                  ] } }
            ] },
            "output_types": { "types": [
                { "kind": "primitive", "name": "std::string::String", "description": "UTF-8" },
                { "kind": "primitive", "name": "u32", "description": "u32" },
                { "kind": "primitive", "name": "std::vec::Vec", "description": "Vec",
                  "parameters": [{ "name": "T" }] },
                { "kind": "struct", "name": "Item",
                  "fields": { "named": [
                    { "name": "id", "type": { "name": "u32" }, "required": true },
                    { "name": "label", "type": { "name": "std::string::String" }, "required": true }
                  ] } },
                { "kind": "struct", "name": "FlatTree",
                  "parameters": [{ "name": "T" }],
                  "fields": { "named": [
                    { "name": "value", "type": { "name": "T" }, "required": true, "flattened": true },
                    { "name": "children",
                      "type": { "name": "std::vec::Vec",
                                "arguments": [{ "name": "FlatTree", "arguments": [{ "name": "T" }] }] },
                      "required": true }
                  ] } }
            ] }
        }"#;
        let schema: crate::Schema = serde_json::from_str(schema_json).unwrap();
        let py = super::generate(schema, &Config::default()).expect("codegen must terminate");
        // The monomorphized class is named with both struct path and
        // arg leaf (no namespace prefix on either here, both are
        // top-level). The flatten pulled Item's id+label up; the
        // recursive children list refers back to the same class.
        assert!(py.contains("class FlatTreeItem"));
        assert!(py.contains("children: list[FlatTreeItem]"));
        assert!(py.contains("    id: int"));
        assert!(py.contains("    label: str"));
    }
}
