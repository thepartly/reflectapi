# Defining Types

In this first step, we'll create all the data models for our Pet Store API. ReflectAPI uses derive macros to automatically generate schemas, serialization, and client code from your Rust types.

## The Pet Model

Let's start by defining our core `Pet` type. Create `src/model.rs`:

```rust
# extern crate reflectapi;
# extern crate serde;
# extern crate chrono;
# 
# use reflectapi::{Input, Output};
# use serde::{Serialize, Deserialize};
# 
# #[derive(Debug, Clone, Serialize, Deserialize, Input, Output)]
# #[serde(tag = "type", rename_all = "snake_case")]
# pub enum PetKind {
#     Dog { breed: String },
#     Cat { lives: u8 },
#     Bird { can_talk: bool },
# }
# 
# #[derive(Debug, Clone, Serialize, Deserialize, Input, Output)]
# pub enum Behavior {
#     Calm,
#     Aggressive { level: f64, notes: String },
#     Playful { favorite_toy: Option<String> },
#     Other { description: String },
# }
# 
# fn main() {
use reflectapi::{Input, Output};

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output,
)]
pub struct Pet {
    /// Unique identifier for the pet
    pub id: u32,
    /// Pet's name
    pub name: String,
    /// What kind of animal this pet is
    pub kind: PetKind,
    /// Age in years (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<u8>,
    /// When this pet record was last updated
    #[serde(default = "chrono::Utc::now")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Notable behaviors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviors: Vec<Behavior>,
}

// Test that the struct can be created
let _pet = Pet {
    id: 1,
    name: "Buddy".to_string(),
    kind: PetKind::Dog { breed: "Golden Retriever".to_string() },
    age: Some(3),
    updated_at: chrono::Utc::now(),
    behaviors: vec![Behavior::Calm],
};
# }
```


### Pet Kind Enum

Different types of pets have different attributes. We'll use a tagged enum:

```rust,ignore
# use reflectapi::{Input, Output};
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output,
)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PetKind {
    /// A friendly dog
    Dog {
        /// Dog breed
        breed: String,
    },
    /// An independent cat  
    Cat {
        /// How many lives they have left
        lives: u8,
    },
    /// A colorful bird
    Bird {
        /// Can this bird talk?
        can_talk: bool,
        /// Wingspan in centimeters
        wingspan_cm: Option<u16>,
    },
}
```

This creates a **discriminated union** that serializes as:

```json
// Dog example
{
  "type": "dog",
  "breed": "Golden Retriever"
}

// Cat example  
{
  "type": "cat",
  "lives": 7
}
```

### Behavior Enum

Pets can have various behaviors with different data:

```rust,ignore
# use reflectapi::{Input, Output};
#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output,
)]
pub enum Behavior {
    /// Calm and peaceful
    Calm,
    /// Shows aggressive tendencies
    Aggressive {
        /// Severity level (0.0 to 1.0)
        level: f64,
        /// Additional notes about the aggression
        notes: String,
    },
    /// Playful behavior
    Playful {
        /// Favorite toy
        favorite_toy: Option<String>,
    },
    /// Custom behavior description
    Other {
        /// Description of the behavior
        description: String,
    },
}
```

### Working with Enum Patterns

Here's a simple example demonstrating the enum patterns you just learned:

```rust
/// Demonstrates basic enum patterns for pet behaviors
#[derive(Debug, Clone, PartialEq)]
pub enum Behavior {
    Calm,
    Aggressive { level: f64, notes: String },
    Playful { favorite_toy: Option<String> },
    Other { description: String },
}

fn main() {
    // Test basic enum construction and matching
    let calm_behavior = Behavior::Calm;
    let aggressive_behavior = Behavior::Aggressive {
        level: 0.7,
        notes: "Protective of food".to_string(),
    };

    match calm_behavior {
        Behavior::Calm => println!("Pet is calm"),
        Behavior::Aggressive { level, .. } => println!("Aggression level: {}", level),
        _ => println!("Other behavior"),
    }

    // Test enum comparison
    assert_eq!(calm_behavior, Behavior::Calm);

    // Test extracting data from enum variants
    if let Behavior::Aggressive { level, notes } = aggressive_behavior {
        assert_eq!(level, 0.7);
        assert_eq!(notes, "Protective of food");
    }
}
```

## Request and Response Types

Now let's define the types for our API operations. Create `src/api_types.rs`:

```rust,ignore,ignore
use reflectapi::{Input, Output};
use crate::model::{Pet, PetKind, Behavior};

// ============================================================================
// Health Check
// ============================================================================

#[derive(serde::Serialize, Output)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub timestamp: String,
}

// ============================================================================
// Authentication
// ============================================================================

#[derive(serde::Deserialize, Input)]
pub struct ApiHeaders {
    /// API key for authentication
    #[serde(default)]
    pub authorization: String,
}

// ============================================================================
// List Pets
// ============================================================================

#[derive(serde::Deserialize, Input)]
pub struct ListPetsRequest {
    /// Maximum number of pets to return
    #[serde(default)]
    pub limit: Option<u8>,
    /// Pagination cursor for next page
    #[serde(default)]
    pub cursor: Option<String>,
    /// Filter by pet kind
    #[serde(default)]
    pub kind_filter: Option<String>,
}

#[derive(serde::Serialize, Output)]
pub struct PaginatedPets {
    /// List of pets for this page
    pub pets: Vec<Pet>,
    /// Cursor for the next page (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Total number of pets available
    pub total_count: u32,
}

// ============================================================================
// Create Pet
// ============================================================================

#[derive(serde::Deserialize, Input)]
pub struct CreatePetRequest {
    /// Pet's name (required)
    pub name: String,
    /// What kind of animal (required)
    pub kind: PetKind,
    /// Age in years (optional)
    #[serde(default)]
    pub age: Option<u8>,
    /// Initial behaviors (optional)
    #[serde(default)]
    pub behaviors: Vec<Behavior>,
}

#[derive(serde::Serialize, Output)]
pub struct CreatePetResponse {
    /// The newly created pet
    pub pet: Pet,
    /// Success message
    pub message: String,
}

// ============================================================================
// Update Pet
// ============================================================================

#[derive(serde::Deserialize, Input)]
pub struct UpdatePetRequest {
    /// Pet ID to update
    pub id: u32,
    /// New name (optional)
    #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
    pub name: reflectapi::Option<String>,
    /// New kind (optional)  
    #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
    pub kind: reflectapi::Option<PetKind>,
    /// New age (optional, can be set to null)
    #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
    pub age: reflectapi::Option<u8>,
    /// New behaviors (optional, can be set to empty)
    #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
    pub behaviors: reflectapi::Option<Vec<Behavior>>,
}

// ============================================================================
// Get Pet
// ============================================================================

#[derive(serde::Deserialize, Input)]
pub struct GetPetRequest {
    /// Pet ID to retrieve
    pub id: u32,
}

// ============================================================================
// Delete Pet  
// ============================================================================

#[derive(serde::Deserialize, Input)]
pub struct DeletePetRequest {
    /// Pet ID to delete
    pub id: u32,
}

#[derive(serde::Serialize, Output)]
pub struct DeletePetResponse {
    /// Success message
    pub message: String,
    /// The deleted pet (for confirmation)
    pub deleted_pet: Pet,
}
```

## The Power of ReflectAPI's Option Type

Notice we're using `reflectapi::Option<T>` for some update fields. This provides **three-state logic**:

- **Undefined**: Field not provided in request (no change)
- **Null**: Explicitly set to null (clear the field)  
- **Some(value)**: Set to a specific value

```json
{
  "id": 123,
  "name": "Fluffy",        // Set name to "Fluffy"
  "age": null,             // Clear the age field
  // behaviors not provided // Leave behaviors unchanged
}
```

This is more powerful than regular `Option<T>` which only has two states.

## Error Types

Let's define comprehensive error types for our API:

```rust,ignore,ignore
use reflectapi::{Output, StatusCode};

// ============================================================================
// Authentication Errors
// ============================================================================

#[derive(serde::Serialize, Output)]
pub struct UnauthorizedError {
    pub message: String,
}

impl StatusCode for UnauthorizedError {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::UNAUTHORIZED
    }
}

// ============================================================================
// Pet Operation Errors
// ============================================================================

#[derive(serde::Serialize, Output)]
pub enum PetError {
    /// Pet not found
    NotFound {
        pet_id: u32,
    },
    /// Pet name already exists
    NameConflict {
        name: String,
    },
    /// Invalid input data
    ValidationError {
        field: String,
        message: String,
    },
    /// Not authorized to perform this operation
    Unauthorized {
        message: String,
    },
    /// Internal server error
    InternalError {
        message: String,
    },
}

impl StatusCode for PetError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            PetError::NotFound { .. } => http::StatusCode::NOT_FOUND,
            PetError::NameConflict { .. } => http::StatusCode::CONFLICT,
            PetError::ValidationError { .. } => http::StatusCode::BAD_REQUEST,
            PetError::Unauthorized { .. } => http::StatusCode::UNAUTHORIZED,
            PetError::InternalError { .. } => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// Conversion from authentication errors
impl From<UnauthorizedError> for PetError {
    fn from(err: UnauthorizedError) -> Self {
        PetError::Unauthorized {
            message: err.message,
        }
    }
}
```

## Application State

Finally, let's define our application state in `src/state.rs`:

```rust,ignore,ignore
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::model::Pet;

#[derive(Debug)]
pub struct AppState {
    /// In-memory pet storage (in a real app, use a database)
    pub pets: Mutex<HashMap<u32, Pet>>,
    /// Next pet ID to assign
    pub next_id: Mutex<u32>,
    /// Valid API keys
    pub api_keys: Vec<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            pets: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            api_keys: vec![
                "demo-api-key".to_string(),
                "test-key-123".to_string(),
            ],
        }
    }
    
    pub fn next_pet_id(&self) -> u32 {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        id
    }
    
    pub fn is_valid_api_key(&self, key: &str) -> bool {
        !key.is_empty() && self.api_keys.contains(&key.to_string())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
```

## Update Your main.rs

Update your `src/main.rs` to include the new modules:

```rust,ignore,ignore
mod model;
mod api_types;
mod state;

use state::AppState;
use std::sync::Arc;

fn main() {
    println!("Pet Store API types defined!");
    println!("Next: Create the API endpoints");
}
```

## What You've Accomplished

You've now defined:

✅ **Core data models** (`Pet`, `PetKind`, `Behavior`) with proper derive macros  
✅ **Request/response types** for all CRUD operations  
✅ **Three-state option handling** for partial updates  
✅ **Comprehensive error types** with HTTP status codes  
✅ **Application state** management  

The `Input` and `Output` derive macros automatically:
- Generate JSON schema information
- Enable serialization/deserialization  
- Create TypeScript type definitions
- Generate Python Pydantic models
- Produce OpenAPI documentation

## Understanding the Generated Schema

ReflectAPI analyzes your types and creates rich schemas. For example, your `Pet` type generates:

- **TypeScript**: `interface Pet { id: number; name: string; kind: PetKind; ... }`
- **Python**: `class Pet(BaseModel): id: int; name: str; kind: PetKind; ...`
- **OpenAPI**: Full JSON schema with validation rules and documentation

## Next Steps

Now that we have our data models, let's [create the API endpoints](./creating-endpoints.md) that use these types with ReflectAPI's canonical handler signature!
