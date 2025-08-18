use crate::codegen::python::{
    generate_python_client, render::RenderConfig, semantic::PyCodegenConfig, PipelineConfig,
};
/// Simple test to verify the IR pipeline works with minimal schema
use anyhow::Result;
use reflectapi_schema::Schema;

#[test]
fn test_minimal_pipeline() -> Result<()> {
    let mut schema = Schema::new();
    schema.name = "MinimalAPI".to_string();
    schema.description = "Minimal API for testing".to_string();

    let config = PipelineConfig {
        package_name: "minimal_client".to_string(),
        codegen: PyCodegenConfig {
            generate_async: false,
            generate_sync: true,
            generate_testing: false,
            generate_pyi_stubs: false,
            base_url: None,
            package_name: "minimal_client".to_string(),
        },
        render: RenderConfig::default(),
        format_code: false,
        validate_code: false,
    };

    let generated_code = generate_python_client(schema, config)?;

    println!("Generated code:\n{}", generated_code);
    
    // Verify basic structure
    assert!(generated_code.contains("from __future__ import annotations"), 
            "Missing future import");
    
    // The package name is now in the module, not necessarily visible in generated code
    // unless there's actual content
    assert!(!generated_code.is_empty(), "Generated code is empty");

    Ok(())
}
