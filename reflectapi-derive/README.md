# ReflectAPI Derive Macros

Procedural macros for automatically implementing ReflectAPI traits.

## Overview

This crate provides derive macros for the `Input` and `Output` traits that are core to ReflectAPI. These macros automatically generate the necessary implementations to make your types work seamlessly with ReflectAPI's schema generation and code generation pipeline.

## Usage

Add this to your `Cargo.toml` (usually via the main `reflectapi` crate):

```toml
[dependencies]
reflectapi = "0.15"
```

Then derive the traits on your types:

```rust
use reflectapi::{Input, Output};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Input, Output)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
}

#[derive(Serialize, Deserialize, Input)]
struct CreateUserRequest {
    name: String,
    email: Option<String>,
}
```

## Supported Types

The derive macros support:

- **Structs**: Named fields, tuple structs, and unit structs
- **Enums**: All serde tagging strategies (external, internal, adjacent, untagged)
- **Generics**: Generic types with proper constraint handling
- **Options**: Optional fields using `Option<T>`
- **Collections**: `Vec<T>`, `HashMap<K, V>`, `HashSet<T>`, etc.
- **External types**: Integration with `chrono`, `uuid`, `url`, etc.

## Attributes

The derive macros respect serde attributes and add ReflectAPI-specific ones:

### Field Attributes

```rust
#[derive(Input, Output)]
struct Example {
    // Skip this field from the API
    #[reflectapi(skip)]
    internal_field: String,
    
    // Only include in input types
    #[reflectapi(input_only)]
    password: String,
    
    // Only include in output types  
    #[reflectapi(output_only)]
    created_at: DateTime<Utc>,
    
    // Transform the type in the schema
    #[reflectapi(transform = "string")]
    complex_field: CustomType,
}
```

### Type Attributes

```rust
// Skip deriving for certain directions
#[derive(Input)]
#[reflectapi(skip_output)]
struct InputOnlyType {
    data: String,
}

// Add documentation
#[derive(Input, Output)]
#[reflectapi(description = "A user in the system")]
struct User {
    id: u32,
    name: String,
}
```

## Generated Code

The derive macros generate implementations of the `Input` and `Output` traits:

```rust
impl Input for User {
    fn reflectapi_input_type(schema: &mut Typespace) -> TypeReference {
        // Generated implementation that builds the schema
    }
}

impl Output for User {
    fn reflectapi_output_type(schema: &mut Typespace) -> TypeReference {
        // Generated implementation that builds the schema
    }
}
```

These implementations automatically:
- Register the type in the schema
- Handle generic parameters
- Respect serde attributes
- Apply ReflectAPI-specific transformations

