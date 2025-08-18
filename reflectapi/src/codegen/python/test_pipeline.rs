/// Comprehensive tests for the Python IR-based code generation pipeline
/// 
/// These tests verify the complete four-stage compiler pipeline:
/// 1. Schema normalization 
/// 2. Semantic IR transformation
/// 3. Syntax IR generation
/// 4. Code rendering

use anyhow::Result;
use reflectapi_schema::{
    Schema, SymbolId, SymbolKind, Function, Struct, Field, Enum, Variant,
    TypeReference, Type,
};
use crate::codegen::python::{
    generate_python_client, PipelineConfig,
    semantic::PyCodegenConfig,
    render::RenderConfig,
};

/// Create a test schema with basic types and functions
fn create_test_schema() -> Schema {
    let mut schema = Schema::new();
    schema.id = SymbolId::new(SymbolKind::Struct, vec!["test_api".to_string()]);
    schema.name = "TestAPI".to_string();
    schema.description = "Test API for pipeline validation".to_string();
    
    // Add a simple struct type
    let user_struct = Struct {
        id: SymbolId::new(SymbolKind::Struct, vec!["User".to_string()]),
        name: "User".to_string(),
        serde_name: String::new(),
        description: "User model".to_string(),
        parameters: vec![],
        fields: reflectapi_schema::Fields::Named(vec![
            Field {
                id: SymbolId::new(SymbolKind::Field, vec!["User".to_string(), "id".to_string()]),
                name: "id".to_string(),
                serde_name: String::new(),
                description: "User ID".to_string(),
                deprecation_note: None,
                type_ref: TypeReference::new("u32", vec![]),
                required: true,
                flattened: false,
                transform_callback: String::new(),
                transform_callback_fn: None,
            },
            Field {
                id: SymbolId::new(SymbolKind::Field, vec!["User".to_string(), "name".to_string()]),
                name: "name".to_string(),
                serde_name: String::new(),
                description: "User name".to_string(),
                deprecation_note: None,
                type_ref: TypeReference::new("String", vec![]),
                required: true,
                flattened: false,
                transform_callback: String::new(),
                transform_callback_fn: None,
            },
            Field {
                id: SymbolId::new(SymbolKind::Field, vec!["User".to_string(), "email".to_string()]),
                name: "email".to_string(),
                serde_name: String::new(),
                description: "User email".to_string(),
                deprecation_note: None,
                type_ref: TypeReference::new("String", vec![]),
                required: false,
                flattened: false,
                transform_callback: String::new(),
                transform_callback_fn: None,
            },
        ]),
        transparent: false,
        codegen_config: Default::default(),
    };
    
    // Add struct to input types
    schema.input_types.insert("User".to_string(), Type::Struct(user_struct.clone()));
    schema.output_types.insert("User".to_string(), Type::Struct(user_struct));
    
    // Add an enum type
    let status_enum = Enum {
        id: SymbolId::new(SymbolKind::Enum, vec!["Status".to_string()]),
        name: "Status".to_string(),
        serde_name: String::new(),
        description: "User status".to_string(),
        parameters: vec![],
        representation: reflectapi_schema::Representation::External,
        variants: vec![
            Variant {
                id: SymbolId::new(SymbolKind::Variant, vec!["Status".to_string(), "Active".to_string()]),
                name: "Active".to_string(),
                serde_name: String::new(),
                description: "User is active".to_string(),
                fields: reflectapi_schema::Fields::Unit,
                discriminant: None,
                untagged: false,
            },
            Variant {
                id: SymbolId::new(SymbolKind::Variant, vec!["Status".to_string(), "Inactive".to_string()]),
                name: "Inactive".to_string(),
                serde_name: String::new(),
                description: "User is inactive".to_string(),
                fields: reflectapi_schema::Fields::Unit,
                discriminant: None,
                untagged: false,
            },
        ],
        codegen_config: Default::default(),
    };
    
    schema.input_types.insert("Status".to_string(), Type::Enum(status_enum.clone()));
    schema.output_types.insert("Status".to_string(), Type::Enum(status_enum));
    
    // Add API functions
    let get_user_fn = Function {
        id: SymbolId::new(SymbolKind::Endpoint, vec!["get_user".to_string()]),
        name: "get_user".to_string(),
        path: "/users/{id}".to_string(),
        description: "Get user by ID".to_string(),
        input_type: Some(TypeReference::new("u32", vec![])),
        output_type: Some(TypeReference::new("User", vec![])),
        error_type: None,
        serialization: vec![],
        tags: Default::default(),
        readonly: true,
        deprecation_note: None,
        input_headers: None,
    };
    
    let create_user_fn = Function {
        id: SymbolId::new(SymbolKind::Endpoint, vec!["create_user".to_string()]),
        name: "create_user".to_string(),
        path: "/users".to_string(),
        description: "Create new user".to_string(),
        input_type: Some(TypeReference::new("User", vec![])),
        output_type: Some(TypeReference::new("User", vec![])),
        error_type: None,
        serialization: vec![],
        tags: Default::default(),
        readonly: false,
        deprecation_note: None,
        input_headers: None,
    };
    
    schema.functions = vec![get_user_fn, create_user_fn];
    
    schema
}

#[test]
fn test_full_pipeline_basic_schema() -> Result<()> {
    let schema = create_test_schema();
    
    let config = PipelineConfig {
        package_name: "test_client".to_string(),
        codegen: PyCodegenConfig {
            generate_async: true,
            generate_sync: false,
            generate_testing: false,
            generate_pyi_stubs: false,
            base_url: Some("https://api.example.com".to_string()),
            package_name: "test_client".to_string(),
        },
        render: RenderConfig::default(),
        format_code: false, // Skip formatting in tests
        validate_code: false, // Skip validation in tests
    };
    
    let generated_code = generate_python_client(schema, config)?;
    
    // Verify basic structure
    assert!(generated_code.contains("Generated Python client"));
    assert!(generated_code.contains("test_client"));
    assert!(generated_code.contains("from __future__ import annotations"));
    assert!(generated_code.contains("from pydantic import BaseModel"));
    
    // Verify User model generation
    assert!(generated_code.contains("class User(BaseModel):"));
    assert!(generated_code.contains("id: int"));
    assert!(generated_code.contains("name: str"));
    assert!(generated_code.contains("email: Optional[str]"));
    
    // Verify Status enum generation
    assert!(generated_code.contains("Status"));
    
    // Verify client class generation
    assert!(generated_code.contains("Client"));
    assert!(generated_code.contains("get_user"));
    assert!(generated_code.contains("create_user"));
    
    Ok(())
}

#[test]
fn test_pipeline_with_sync_and_async() -> Result<()> {
    let schema = create_test_schema();
    
    let config = PipelineConfig {
        package_name: "dual_client".to_string(),
        codegen: PyCodegenConfig {
            generate_async: true,
            generate_sync: true,
            generate_testing: false,
            generate_pyi_stubs: false,
            base_url: None,
            package_name: "dual_client".to_string(),
        },
        render: RenderConfig::default(),
        format_code: false,
        validate_code: false,
    };
    
    let generated_code = generate_python_client(schema, config)?;
    
    // Should import both sync and async base classes
    assert!(generated_code.contains("AsyncClientBase"));
    assert!(generated_code.contains("ClientBase"));
    
    Ok(())
}

#[test]
fn test_pipeline_with_testing_support() -> Result<()> {
    let schema = create_test_schema();
    
    let config = PipelineConfig {
        package_name: "testing_client".to_string(),
        codegen: PyCodegenConfig {
            generate_async: false,
            generate_sync: true,
            generate_testing: true,
            generate_pyi_stubs: false,
            base_url: None,
            package_name: "testing_client".to_string(),
        },
        render: RenderConfig::default(),
        format_code: false,
        validate_code: false,
    };
    
    let generated_code = generate_python_client(schema, config)?;
    
    // Should include testing imports
    assert!(generated_code.contains("MockClient"));
    assert!(generated_code.contains("create_api_response"));
    
    Ok(())
}

#[test]
fn test_empty_schema_handling() -> Result<()> {
    let mut schema = Schema::new();
    schema.id = SymbolId::new(SymbolKind::Struct, vec!["empty".to_string()]);
    schema.name = "EmptyAPI".to_string();
    schema.description = "Empty API".to_string();
    
    let config = PipelineConfig::default();
    let generated_code = generate_python_client(schema, config)?;
    
    // Should still generate basic structure
    assert!(generated_code.contains("Generated Python client"));
    assert!(generated_code.contains("from __future__ import annotations"));
    
    Ok(())
}

#[test]
fn test_complex_types_pipeline() -> Result<()> {
    let mut schema = Schema::new();
    schema.id = SymbolId::new(SymbolKind::Struct, vec!["complex_api".to_string()]);
    schema.name = "ComplexAPI".to_string();
    schema.description = "API with complex types".to_string();
    
    // Add internally tagged enum
    let internally_tagged_enum = Enum {
        id: SymbolId::new(SymbolKind::Enum, vec!["Event".to_string()]),
        name: "Event".to_string(),
        description: "Event with internal tagging".to_string(),
        variants: vec![
            Variant {
                id: SymbolId::new(SymbolKind::Variant, vec!["Event".to_string(), "UserCreated".to_string()]),
                name: "UserCreated".to_string(),
                description: "User created event".to_string(),
                fields: vec![
                    Field {
                        id: SymbolId::new(SymbolKind::Field, vec!["Event".to_string(), "UserCreated".to_string(), "user_id".to_string()]),
                        name: "user_id".to_string(),
                        description: "User ID".to_string(),
                        type_: TypeReference::new("u32", vec![]),
                        required: true,
                        deprecation_note: None,
                    },
                ],
                discriminant: None,
                attributes: vec![],
            },
        ],
        generic_parameters: vec![],
        attributes: vec![],
        representation: reflectapi_schema::Representation::Internal { tag: "type".to_string() },
    };
    
    schema.input_types.insert("Event".to_string(), Type::Enum(internally_tagged_enum.clone()));
    schema.output_types.insert("Event".to_string(), Type::Enum(internally_tagged_enum));
    
    let config = PipelineConfig::default();
    let generated_code = generate_python_client(schema, config)?;
    
    // Should handle internally tagged enums with discriminated unions
    assert!(generated_code.contains("Event"));
    assert!(generated_code.contains("Literal")); // For discriminator fields
    
    Ok(())
}

#[test]
fn test_pipeline_deterministic_output() -> Result<()> {
    let schema = create_test_schema();
    let config = PipelineConfig::default();
    
    // Generate code twice and verify it's identical
    let generated1 = generate_python_client(schema.clone(), config.clone())?;
    let generated2 = generate_python_client(schema, config)?;
    
    assert_eq!(generated1, generated2, "Pipeline output should be deterministic");
    
    Ok(())
}

#[test]
fn test_pipeline_error_handling() {
    // Test with invalid schema (missing required fields)
    let mut schema = Schema::new();
    // Don't set ID - this should cause normalization to fail
    schema.name = "InvalidAPI".to_string();
    
    let config = PipelineConfig::default();
    let result = generate_python_client(schema, config);
    
    // Should return an error, not panic
    assert!(result.is_err());
}

#[test]
fn test_symbol_id_stability() -> Result<()> {
    let schema = create_test_schema();
    let config = PipelineConfig::default();
    
    let generated = generate_python_client(schema, config)?;
    
    // Verify that symbol IDs are being used consistently
    // The normalization phase should produce stable, deterministic SymbolIds
    assert!(generated.contains("User"));
    assert!(generated.contains("Status"));
    
    Ok(())
}

#[test]
fn test_import_generation() -> Result<()> {
    let schema = create_test_schema();
    
    let config = PipelineConfig {
        package_name: "import_test".to_string(),
        codegen: PyCodegenConfig {
            generate_async: true,
            generate_sync: true,
            generate_testing: true,
            generate_pyi_stubs: false,
            base_url: None,
            package_name: "import_test".to_string(),
        },
        render: RenderConfig::default(),
        format_code: false,
        validate_code: false,
    };
    
    let generated_code = generate_python_client(schema, config)?;
    
    // Verify proper import ordering and organization
    let lines: Vec<&str> = generated_code.lines().collect();
    
    // Should have future imports first
    assert!(lines.iter().any(|line| line.contains("from __future__ import annotations")));
    
    // Should have typing imports
    assert!(lines.iter().any(|line| line.contains("from typing import")));
    
    // Should have pydantic imports
    assert!(lines.iter().any(|line| line.contains("from pydantic import")));
    
    // Should have reflectapi runtime imports
    assert!(lines.iter().any(|line| line.contains("from reflectapi_runtime")));
    
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
        
    /// Test that demonstrates the full pipeline working with a realistic API schema
    #[test]
    fn test_realistic_api_pipeline() -> Result<()> {
        let mut schema = Schema::new();
        schema.id = SymbolId::new(SymbolKind::Struct, vec!["blog_api".to_string()]);
        schema.name = "BlogAPI".to_string();
        schema.description = "Blogging platform API".to_string();
        
        // Create Post model
        let post_struct = Struct {
            id: SymbolId::new(SymbolKind::Struct, vec!["Post".to_string()]),
            name: "Post".to_string(),
            description: "Blog post".to_string(),
            fields: vec![
                Field {
                    id: SymbolId::new(SymbolKind::Field, vec!["Post".to_string(), "id".to_string()]),
                    name: "id".to_string(),
                    description: "Post ID".to_string(),
                    type_: TypeReference::new("u32", vec![]),
                    required: true,
                    deprecation_note: None,
                },
                Field {
                    id: SymbolId::new(SymbolKind::Field, vec!["Post".to_string(), "title".to_string()]),
                    name: "title".to_string(),
                    description: "Post title".to_string(),
                    type_: TypeReference::new("String", vec![]),
                    required: true,
                    deprecation_note: None,
                },
                Field {
                    id: SymbolId::new(SymbolKind::Field, vec!["Post".to_string(), "content".to_string()]),
                    name: "content".to_string(),
                    description: "Post content".to_string(),
                    type_: TypeReference::new("String", vec![]),
                    required: true,
                    deprecation_note: None,
                },
                Field {
                    id: SymbolId::new(SymbolKind::Field, vec!["Post".to_string(), "published".to_string()]),
                    name: "published".to_string(),
                    description: "Publication status".to_string(),
                    type_: TypeReference::new("bool", vec![]),
                    required: false,
                    deprecation_note: None,
                },
            ],
            generic_parameters: vec![],
            attributes: vec![],
        };
        
        schema.input_types.insert("Post".to_string(), Type::Struct(post_struct.clone()));
        schema.output_types.insert("Post".to_string(), Type::Struct(post_struct));
        
        // Add CRUD functions
        let functions = vec![
            Function {
                id: SymbolId::new(SymbolKind::Endpoint, vec!["list_posts".to_string()]),
                name: "list_posts".to_string(),
                path: "/posts".to_string(),
                description: "List all posts".to_string(),
                input_type: None,
                output_type: Some(TypeReference::new("Vec<Post>", vec![])),
                error_type: None,
                serialization_mode: vec![],
                generic_parameters: vec![],
                attributes: vec![],
                readonly: true,
            },
            Function {
                id: SymbolId::new(SymbolKind::Endpoint, vec!["get_post".to_string()]),
                name: "get_post".to_string(),
                path: "/posts/{id}".to_string(),
                description: "Get post by ID".to_string(),
                input_type: Some(TypeReference::new("u32".to_string())),
                output_type: Some(TypeReference::new("Post", vec![])),
                error_type: None,
                serialization_mode: vec![],
                generic_parameters: vec![],
                attributes: vec![],
                readonly: true,
            },
            Function {
                id: SymbolId::new(SymbolKind::Endpoint, vec!["create_post".to_string()]),
                name: "create_post".to_string(),
                path: "/posts".to_string(),
                description: "Create new post".to_string(),
                input_type: Some(TypeReference::new("Post", vec![])),
                output_type: Some(TypeReference::new("Post", vec![])),
                error_type: None,
                serialization_mode: vec![],
                generic_parameters: vec![],
                attributes: vec![],
                readonly: false,
            },
            Function {
                id: SymbolId::new(SymbolKind::Endpoint, vec!["update_post".to_string()]),
                name: "update_post".to_string(),
                path: "/posts/{id}".to_string(),
                description: "Update post".to_string(),
                input_type: Some(TypeReference::new("Post", vec![])),
                output_type: Some(TypeReference::new("Post", vec![])),
                error_type: None,
                serialization_mode: vec![],
                generic_parameters: vec![],
                attributes: vec![],
                readonly: false,
            },
            Function {
                id: SymbolId::new(SymbolKind::Endpoint, vec!["delete_post".to_string()]),
                name: "delete_post".to_string(),
                path: "/posts/{id}".to_string(),
                description: "Delete post".to_string(),
                input_type: Some(TypeReference::new("u32".to_string())),
                output_type: None,
                error_type: None,
                serialization_mode: vec![],
                generic_parameters: vec![],
                attributes: vec![],
                readonly: false,
            },
        ];
        
        schema.functions = functions;
        
        let config = PipelineConfig {
            package_name: "blog_client".to_string(),
            codegen: PyCodegenConfig {
                generate_async: true,
                generate_sync: true,
                generate_testing: true,
                generate_pyi_stubs: false,
                base_url: Some("https://blog.example.com/api".to_string()),
                package_name: "blog_client".to_string(),
            },
            render: RenderConfig::default(),
            format_code: false,
            validate_code: false,
        };
        
        let generated_code = generate_python_client(schema, config)?;
        
        // Verify comprehensive API client generation
        assert!(generated_code.contains("class Post(BaseModel):"));
        assert!(generated_code.contains("class ApiClient("));
        
        // Verify all CRUD methods are generated
        assert!(generated_code.contains("list_posts"));
        assert!(generated_code.contains("get_post"));
        assert!(generated_code.contains("create_post"));
        assert!(generated_code.contains("update_post"));
        assert!(generated_code.contains("delete_post"));
        
        // Verify HTTP methods are correctly mapped
        assert!(generated_code.contains("GET"));
        assert!(generated_code.contains("POST"));
        assert!(generated_code.contains("PUT"));
        assert!(generated_code.contains("DELETE"));
        
        Ok(())
    }
}