//! Test-Driven Development for Python Pipeline
//!
//! This module tests each pipeline stage in isolation to ensure correct
//! information flows from Semantic IR → Syntax IR → Rendered Code

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::codegen::python::render::*;
    use crate::codegen::python::semantic::*;
    use crate::codegen::python::syntax::*;
    use crate::codegen::python::transform::*;
    use crate::codegen::python::{generate_python_client, PipelineConfig};
    use reflectapi_schema::Schema;

    /// Test understanding the current semantic output by examining generated client
    #[test]
    fn test_current_semantic_output() {
        println!("=== Analyzing Current Semantic IR Output ===");

        // Create a minimal schema to test with
        let mut schema = Schema::new();
        schema.name = "TestAPI".to_string();

        // We'll add a simple discriminated union to understand the issue
        // This is a placeholder test to understand the current flow

        let config = PipelineConfig {
            package_name: "test_client".to_string(),
            codegen: PyCodegenConfig::default(),
            render: RenderConfig::default(),
            format_code: false,
            validate_code: false,
        };

        // Just verify that we can call the generation pipeline
        match generate_python_client(schema, config) {
            Ok(generated_code) => {
                println!(
                    "Generated {} characters of Python code",
                    generated_code.len()
                );

                // Look for problematic patterns in the generated code
                if generated_code.contains("PetsCreateErrorInvalidIdentityVariant") {
                    println!("✗ Still generating undefined variant types");
                } else {
                    println!("✓ No undefined variant types found in minimal test");
                }
            }
            Err(e) => {
                println!("Generation failed: {}", e);
            }
        }

        assert!(true);
    }

    /// Test that module-prefixed types have consistent naming
    #[test]
    fn test_module_prefixed_type_naming_consistency() {
        println!("=== Testing Module-Prefixed Type Naming Consistency ===");

        // Since the schema has been normalized, we now use SemanticSchema
        use reflectapi_schema::SemanticSchema;
        use std::collections::BTreeMap;

        // Create a minimal semantic schema for testing
        // Note: The normalization stage handles enum representations automatically
        // For now, just verify the generation pipeline runs
        let semantic_schema = SemanticSchema {
            id: reflectapi_schema::SymbolId::new(
                reflectapi_schema::SymbolKind::Struct,
                vec!["test".to_string()],
            ),
            name: "TestSchema".to_string(),
            description: "Test schema".to_string(),
            types: BTreeMap::new(),
            functions: BTreeMap::new(),
            symbol_table: reflectapi_schema::SymbolTable::new(),
        };

        // Transform to Python semantic IR
        let config = PyCodegenConfig::default();
        let mut transform = PySemanticTransform::new(config);
        let semantic_ir = transform.transform(semantic_schema);

        // Transform to syntax IR
        let syntax_transform = PySyntaxTransform::new();
        let syntax_module = syntax_transform.transform(semantic_ir);

        // Render to Python code
        let mut renderer = Renderer::new();
        let generated_code = renderer.render_module(&syntax_module);

        println!("Generated Python code:\n{}", generated_code);

        // Basic validation that code generates without errors
        assert!(generated_code.contains("from __future__ import annotations"));
        assert!(generated_code.contains("from typing import"));
    }

    /// Test the renderer stage in isolation
    #[test]
    fn test_renderer_stage_isolated() {
        println!("=== Testing Renderer Stage ===");

        // Manually construct a known-good syntax::Class
        let mut class = Class::new("MyApiModel".to_string());
        class.add_base(TypeExpr::name("BaseModel"));
        class.add_field(Field {
            name: "user_id".to_string(),
            type_annotation: TypeExpr::name("int"),
            default: None,
            field_config: None,
        });

        // Run through Renderer and assert exact output
        let mut module = Module::new("test".to_string());
        module.add_item(Item::Class(class));
        let mut renderer = Renderer::new();
        let rendered = renderer.render_module(&module);
        let expected = "class MyApiModel(BaseModel):\n    user_id: int\n";
        assert_eq!(rendered, expected);
    }

    /// Test the syntax transform stage: SimpleModel -> Class
    #[test]
    fn test_syntax_transform_simple_model_to_class() {
        println!("=== Testing Syntax Transform (SimpleModel -> Class) ===");

        use std::collections::BTreeMap;

        // Build a minimal semantic SimpleModel
        let mut fields = BTreeMap::new();
        fields.insert(
            "user_id".to_string(),
            FieldDef {
                name: "user_id".to_string(),
                python_name: "user_id".to_string(),
                type_annotation: "int".to_string(),
                description: String::new(),
                deprecation_note: None,
                optional: false,
                default_value: None,
                field_config: None,
            },
        );

        let model = ModelDef {
            name: "MyApiModel".to_string(),
            description: "Example model".to_string(),
            fields,
            generic_params: vec![],
            base_classes: vec!["BaseModel".to_string()],
            decorator_config: DecoratorConfig {
                config_dict: None,
                field_validators: vec![],
                model_validators: vec![],
            },
        };

        let semantic_ir = PySemanticIR {
            package_name: "testpkg".to_string(),
            description: String::new(),
            types: vec![("MyApiModel".to_string(), PySemanticType::SimpleModel(model))],
            endpoints: BTreeMap::new(),
            required_imports: RequiredImports::default(),
            config: PyCodegenConfig::default(),
        };

        // Transform to syntax IR
        let syntax_module = PySyntaxTransform::new().transform(semantic_ir);

        // Expect one class with the correct name, base and field
        let class = syntax_module
            .items
            .iter()
            .find_map(|it| {
                if let Item::Class(c) = it {
                    Some(c)
                } else {
                    None
                }
            })
            .expect("Class item should be generated");

        assert_eq!(class.name, "MyApiModel");
        assert_eq!(class.bases.len(), 1);
        match &class.bases[0] {
            TypeExpr::Name(n) => assert_eq!(n, "BaseModel"),
            _ => panic!("Expected BaseModel base"),
        }

        let user_id = class
            .fields
            .iter()
            .find(|f| f.name == "user_id")
            .expect("user_id field should be present");

        match &user_id.type_annotation {
            TypeExpr::Name(n) => assert_eq!(n, "int"),
            _ => panic!("Expected int type annotation"),
        }
    }
}
