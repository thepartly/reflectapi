/// Python IR-based Code Generation Pipeline
///
/// This module implements a four-stage compiler pipeline for Python client generation:
///
/// 1. **Semantic Analysis**: Transform normalized schema into Python-specific semantic decisions
/// 2. **Syntax IR Construction**: Convert semantic decisions into Python AST structures  
/// 3. **Code Generation**: Render syntax IR into formatted Python source code
/// 4. **Post-processing**: Format and validate generated code
///
/// The pipeline separates "what to generate" (semantic) from "how to generate" (syntax),
/// enabling better testing, maintainability, and extensibility.
pub mod naming;
pub mod render;
pub mod semantic;
pub mod syntax;
pub mod transform;

// #[cfg(test)]
// mod test_pipeline;

#[cfg(test)]
mod simple_test;

#[cfg(test)]
mod tdd_test;

use anyhow::{Context, Result};
use reflectapi_schema::{NormalizationPipeline, Schema};
use std::collections::BTreeMap;

use self::{
    render::{RenderConfig, Renderer},
    semantic::{PyCodegenConfig, PySemanticTransform},
    transform::PySyntaxTransform,
};

/// Configuration for the entire Python generation pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Package name for the generated client
    pub package_name: String,

    /// Python-specific code generation options
    pub codegen: PyCodegenConfig,

    /// Code rendering options
    pub render: RenderConfig,

    /// Whether to format generated code with external tools
    pub format_code: bool,

    /// Whether to validate generated code
    pub validate_code: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            package_name: "api_client".to_string(),
            codegen: PyCodegenConfig::default(),
            render: RenderConfig::default(),
            format_code: true,
            validate_code: false,
        }
    }
}

/// Main entry point for Python code generation using the IR pipeline
pub fn generate_python_client(mut schema: Schema, config: PipelineConfig) -> Result<String> {
    // Debug: Confirm this function is being called
    eprintln!("DEBUG: generate_python_client called with IR pipeline");
    // Stage 0: Apply normalization pipeline (type consolidation + naming resolution + circular dependency resolution)
    let normalization_pipeline = NormalizationPipeline::standard();
    normalization_pipeline.run(&mut schema).map_err(|errors| {
        let error_messages: Vec<String> = errors
            .iter()
            .map(|e| format!("Normalization error: {}", e))
            .collect();
        anyhow::anyhow!(
            "Schema normalization failed:\n{}",
            error_messages.join("\n")
        )
    })?;

    // Stage 1: Use the existing normalizer (should work now that cycles are resolved)
    let normalizer = reflectapi_schema::Normalizer::new();
    let semantic_schema = normalizer.normalize(schema).map_err(|errors| {
        let error_messages: Vec<String> = errors
            .iter()
            .map(|e| format!("Semantic transformation error: {}", e))
            .collect();
        anyhow::anyhow!(
            "Semantic transformation failed:\n{}",
            error_messages.join("\n")
        )
    })?;

    // Stage 2: Transform semantic schema into Python semantic decisions
    let mut py_codegen_config = config.codegen.clone();
    py_codegen_config
        .package_name
        .clone_from(&config.package_name);
    let mut py_semantic_transform = PySemanticTransform::new(py_codegen_config);
    let py_semantic_ir = py_semantic_transform.transform(semantic_schema);

    // Stage 3: Convert semantic IR into Python syntax IR
    let py_syntax_transform = PySyntaxTransform::new();
    let py_syntax_ir = py_syntax_transform.transform(py_semantic_ir);

    // Stage 4: Render syntax IR into Python source code
    let mut renderer = Renderer::with_config(config.render);
    let generated_code = renderer.render_module(&py_syntax_ir);

    // Post-processing
    let final_code = if config.format_code {
        format_python_code(&generated_code).unwrap_or(generated_code)
    } else {
        generated_code
    };

    if config.validate_code {
        validate_python_syntax(&final_code).context("Generated Python code validation failed")?;
    }

    Ok(final_code)
}


/// Format Python code using external formatter (black, autopep8, etc.)
fn format_python_code(code: &str) -> Result<String> {
    // Try black first
    if let Ok(formatted) = format_with_black(code) {
        return Ok(formatted);
    }

    // Fall back to autopep8
    if let Ok(formatted) = format_with_autopep8(code) {
        return Ok(formatted);
    }

    // If no formatter available, return original code
    Ok(code.to_string())
}

/// Format code using black
fn format_with_black(code: &str) -> Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("black")
        .args(["--stdin-filename", "generated.py", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn black formatter")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(code.as_bytes())
            .context("Failed to write to black stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to read black output")?;

    if output.status.success() {
        String::from_utf8(output.stdout).context("Black output is not valid UTF-8")
    } else {
        anyhow::bail!(
            "Black formatting failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

/// Format code using autopep8
fn format_with_autopep8(code: &str) -> Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("autopep8")
        .args(["--stdin-display-name", "generated.py", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn autopep8 formatter")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(code.as_bytes())
            .context("Failed to write to autopep8 stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to read autopep8 output")?;

    if output.status.success() {
        String::from_utf8(output.stdout).context("Autopep8 output is not valid UTF-8")
    } else {
        anyhow::bail!(
            "Autopep8 formatting failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

/// Validate Python syntax using the Python interpreter
fn validate_python_syntax(code: &str) -> Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("python3")
        .args(["-m", "py_compile", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn Python syntax validator")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(code.as_bytes())
            .context("Failed to write to Python validator stdin")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to read Python validator output")?;

    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "Python syntax validation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

/// Legacy compatibility wrapper for the existing template-based generator
///
/// This allows gradual migration by providing the same interface while
/// using the new IR pipeline internally.
pub fn generate_legacy_compatible(schema: Schema, package_name: &str) -> Result<String> {
    let config = PipelineConfig {
        package_name: package_name.to_string(),
        ..PipelineConfig::default()
    };

    generate_python_client(schema, config)
}

/// Legacy Config structure for backward compatibility
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
            generate_testing: false,
            base_url: None,
        }
    }
}

impl From<Config> for PipelineConfig {
    fn from(config: Config) -> Self {
        PipelineConfig {
            package_name: config.package_name.clone(),
            codegen: PyCodegenConfig {
                generate_async: config.generate_async,
                generate_sync: config.generate_sync,
                generate_testing: config.generate_testing,
                generate_pyi_stubs: false,
                base_url: config.base_url,
                package_name: config.package_name,
            },
            render: RenderConfig::default(),
            format_code: true,
            validate_code: false,
        }
    }
}

/// Main generate function for backward compatibility with existing tests
pub fn generate(schema: Schema, config: &Config) -> Result<String> {
    eprintln!("DEBUG: generate function called");
    let pipeline_config = PipelineConfig::from(config.clone());
    generate_python_client(schema, pipeline_config)
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_python_codegen_basic() {
        // Simple test to verify the module is working
        let config = PyCodegenConfig::default();
        assert_eq!(config.package_name, "api_client");
    }
    use reflectapi_schema::{Schema, SymbolId, SymbolKind};

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.package_name, "api_client");
        assert!(config.format_code);
        assert!(!config.validate_code);
    }

    #[test]
    fn test_generate_empty_schema() {
        let mut schema = Schema::new();
        schema.name = "test_schema".to_string();
        schema.id = SymbolId::new(SymbolKind::Struct, vec!["test_schema".to_string()]);

        let config = PipelineConfig {
            package_name: "test_client".to_string(),
            format_code: false,   // Skip formatting in tests
            validate_code: false, // Skip validation in tests
            ..PipelineConfig::default()
        };

        let result = generate_python_client(schema, config);
        assert!(result.is_ok(), "Pipeline should handle empty schema");

        let generated = result.unwrap();
        // The package name is in the module metadata, not necessarily in the output
        assert!(!generated.is_empty(), "Should generate some code");
        assert!(generated.contains("from __future__ import annotations"));
    }

    #[test]
    fn test_legacy_compatibility() {
        let mut schema = Schema::new();
        schema.name = "legacy_test".to_string();
        schema.id = SymbolId::new(SymbolKind::Struct, vec!["legacy_test".to_string()]);

        let result = generate_legacy_compatible(schema, "legacy_package");
        assert!(result.is_ok(), "Legacy wrapper should work");

        let generated = result.unwrap();
        assert!(!generated.is_empty(), "Should generate some code for legacy");
    }
}
