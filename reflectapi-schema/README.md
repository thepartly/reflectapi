# ReflectAPI Schema

Core schema representation and transformation utilities for ReflectAPI.

## Overview

This crate provides the fundamental data structures and utilities for representing, manipulating, and transforming ReflectAPI schemas. It serves as the foundation for both schema generation (from Rust types) and code generation (to target languages).

## Core Types

### Schema Structure

The schema system is built around several key types:

- **`Typespace`**: The main container for all type definitions in a schema
- **`TypeDefinition`**: Represents a single type (struct, enum, primitive, etc.)
- **`TypeReference`**: A reference to a type with optional generic parameters
- **`Function`**: Represents an API endpoint/function

### Type Definitions

```rust
use reflectapi_schema::{Typespace, TypeDefinition, Struct, Enum};

// Create a new typespace
let mut typespace = Typespace::new();

// Define a struct type
let user_struct = Struct::new("User")
    .with_field("id", TypeReference::new("u32", vec![]))
    .with_field("name", TypeReference::new("String", vec![]));

// Add to typespace
typespace.insert_type(TypeDefinition::Struct(user_struct));
```

## Schema Transformations

The crate provides utilities for transforming schemas:

### Type Renaming

```rust
use reflectapi_schema::rename::rename_types;

// Rename types matching a pattern
rename_types(&mut schema, "myapp::", "api::");
```

### Type Substitution

```rust
use reflectapi_schema::subst::substitute_types;

// Replace one type with another throughout the schema
substitute_types(&mut schema, "OldType", "NewType");
```

### Schema Validation

```rust
use reflectapi_schema::visit::SchemaVisitor;

// Implement custom validation logic
struct ValidationVisitor;
impl SchemaVisitor for ValidationVisitor {
    fn visit_type(&mut self, type_def: &TypeDefinition) {
        // Custom validation logic
    }
}
```

## Code Generation Support

The schema provides utilities specifically for code generation:

### Type Reference Resolution

```rust
use reflectapi_schema::codegen::resolve_type_reference;

// Resolve a type reference to its full definition
let resolved = resolve_type_reference(&schema, &type_ref);
```

### Dependency Analysis

```rust
use reflectapi_schema::codegen::analyze_dependencies;

// Analyze type dependencies for proper generation order
let deps = analyze_dependencies(&schema);
```

## Features

- **`glob`** - Enable glob pattern matching for type operations
- **`serde`** - Serialize/deserialize schema to JSON (enabled by default)

## Usage in ReflectAPI Pipeline

1. **Schema Building**: The `reflectapi` crate uses this to build schemas from Rust types
2. **Transformation**: Apply renaming, substitution, and validation rules
3. **Code Generation**: The `reflectapi-cli` uses this to generate target language code
4. **Serialization**: Export schemas to JSON for external tools

## Example: Complete Workflow

```rust
use reflectapi_schema::{Typespace, Function, TypeDefinition};

// 1. Build schema (usually done by reflectapi crate)
let mut schema = Typespace::new();

// 2. Add types and functions
let user_type = /* ... create user type ... */;
schema.insert_type(user_type);

let create_user_fn = Function::new("create_user")
    .with_input_type(TypeReference::new("CreateUserRequest", vec![]))
    .with_output_type(TypeReference::new("User", vec![]));
schema.insert_function(create_user_fn);

// 3. Apply transformations
rename_types(&mut schema, "internal::", "api::");

// 4. Serialize for code generation
let schema_json = serde_json::to_string(&schema)?;
```

## Internal Structure

The schema representation is designed to be:
- **Language-agnostic**: Doesn't assume any particular target language
- **Comprehensive**: Can represent complex type systems
- **Transformable**: Supports various transformation operations
- **Serializable**: Can be saved/loaded as JSON

## License

MIT
