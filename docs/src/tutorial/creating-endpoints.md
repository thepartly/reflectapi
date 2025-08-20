# Creating Endpoints

Now that we have our types defined, let's create the API endpoints. ReflectAPI uses a specific handler signature that enables automatic client generation and consistent error handling.

## The Canonical Handler Signature

Every ReflectAPI handler follows this exact pattern:

```rust
async fn handler_name(
    state: Arc<AppState>,
    request: RequestType,
    headers: HeadersType,
) -> Result<ResponseType, ErrorType>
```

This signature is **fixed** and enables ReflectAPI to:
- Generate consistent client code across all languages
- Handle HTTP status codes properly
- Pass request metadata consistently
- Enable middleware integration

## Setting Up the API Module

Create `src/handlers.rs` for our endpoint implementations:

```rust
use std::sync::Arc;
use crate::{
    api_types::*,
    model::*,
    state::AppState,
};

// ============================================================================
// Authentication Helper
// ============================================================================

fn authenticate(headers: &ApiHeaders) -> Result<(), UnauthorizedError> {
    if headers.authorization.is_empty() {
        return Err(UnauthorizedError {
            message: "Missing authorization header".to_string(),
        });
    }
    
    // In a real app, validate the API key properly
    if !headers.authorization.starts_with("Bearer ") {
        return Err(UnauthorizedError {
            message: "Authorization must be 'Bearer <api-key>'".to_string(),
        });
    }
    
    let api_key = headers.authorization.strip_prefix("Bearer ").unwrap();
    if api_key != "demo-api-key" && api_key != "test-key-123" {
        return Err(UnauthorizedError {
            message: "Invalid API key".to_string(),
        });
    }
    
    Ok(())
}

// ============================================================================
// Health Check Endpoint
// ============================================================================

pub async fn health_check(
    _state: Arc<AppState>,
    _request: reflectapi::Empty,
    _headers: reflectapi::Empty,
) -> Result<serde_json::Value, UnauthorizedError> {
    Ok(serde_json::json!({
        "status": "healthy",
        "service": "pet-store-api",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
```

## List Pets Endpoint

```rust
pub async fn list_pets(
    state: Arc<AppState>,
    request: ListPetsRequest,
    headers: ApiHeaders,
) -> Result<PaginatedPets, PetError> {
    // Authenticate the request
    authenticate(&headers)?;
    
    let pets_map = state.pets.lock().unwrap();
    let all_pets: Vec<Pet> = pets_map.values().cloned().collect();
    
    // Apply kind filter if provided
    let filtered_pets: Vec<Pet> = if let Some(kind_filter) = &request.kind_filter {
        all_pets.into_iter()
            .filter(|pet| {
                match (&pet.kind, kind_filter.as_str()) {
                    (PetKind::Dog { .. }, "dog") => true,
                    (PetKind::Cat { .. }, "cat") => true, 
                    (PetKind::Bird { .. }, "bird") => true,
                    _ => false,
                }
            })
            .collect()
    } else {
        all_pets
    };
    
    let total_count = filtered_pets.len() as u32;
    
    // Handle pagination
    let cursor_offset = if let Some(cursor) = &request.cursor {
        cursor.parse::<usize>().unwrap_or(0)
    } else {
        0
    };
    
    let limit = request.limit.unwrap_or(10) as usize;
    let end_index = std::cmp::min(cursor_offset + limit, filtered_pets.len());
    
    let page_pets = filtered_pets.into_iter()
        .skip(cursor_offset)
        .take(limit)
        .collect::<Vec<_>>();
    
    let next_cursor = if end_index < total_count as usize {
        Some(end_index.to_string())
    } else {
        None
    };
    
    Ok(PaginatedPets {
        pets: page_pets,
        next_cursor,
        total_count,
    })
}
```

## Create Pet Endpoint

```rust
pub async fn create_pet(
    state: Arc<AppState>,
    request: CreatePetRequest,
    headers: ApiHeaders,
) -> Result<CreatePetResponse, PetError> {
    // Authenticate the request
    authenticate(&headers)?;
    
    // Validate the pet name
    if request.name.trim().is_empty() {
        return Err(PetError::ValidationError {
            field: "name".to_string(),
            message: "Pet name cannot be empty".to_string(),
        });
    }
    
    if request.name.len() > 50 {
        return Err(PetError::ValidationError {
            field: "name".to_string(),
            message: "Pet name cannot exceed 50 characters".to_string(),
        });
    }
    
    // Check for name conflicts
    let pets = state.pets.lock().unwrap();
    let name_exists = pets.values()
        .any(|pet| pet.name.to_lowercase() == request.name.to_lowercase());
    
    if name_exists {
        return Err(PetError::NameConflict {
            name: request.name,
        });
    }
    drop(pets); // Release the lock
    
    // Create the new pet
    let pet_id = state.next_pet_id();
    let pet = Pet {
        id: pet_id,
        name: request.name.clone(),
        kind: request.kind,
        age: request.age,
        updated_at: chrono::Utc::now(),
        behaviors: request.behaviors,
    };
    
    // Store the pet
    let mut pets = state.pets.lock().unwrap();
    pets.insert(pet_id, pet.clone());
    
    Ok(CreatePetResponse {
        pet,
        message: format!("Pet '{}' created successfully with ID {}", request.name, pet_id),
    })
}
```

## Get Pet Endpoint

```rust
pub async fn get_pet(
    state: Arc<AppState>,
    request: GetPetRequest,
    headers: ApiHeaders,
) -> Result<Pet, PetError> {
    // Authenticate the request
    authenticate(&headers)?;
    
    let pets = state.pets.lock().unwrap();
    
    match pets.get(&request.id) {
        Some(pet) => Ok(pet.clone()),
        None => Err(PetError::NotFound {
            pet_id: request.id,
        }),
    }
}
```

## Update Pet Endpoint

```rust
pub async fn update_pet(
    state: Arc<AppState>,
    request: UpdatePetRequest,
    headers: ApiHeaders,
) -> Result<Pet, PetError> {
    // Authenticate the request
    authenticate(&headers)?;
    
    let mut pets = state.pets.lock().unwrap();
    
    // Get the existing pet
    let pet = pets.get_mut(&request.id)
        .ok_or_else(|| PetError::NotFound {
            pet_id: request.id,
        })?;
    
    // Update fields using ReflectAPI's three-state option
    if let Some(new_name) = request.name.unfold() {
        if let Some(name) = new_name {
            if name.trim().is_empty() {
                return Err(PetError::ValidationError {
                    field: "name".to_string(),
                    message: "Pet name cannot be empty".to_string(),
                });
            }
            
            if name.len() > 50 {
                return Err(PetError::ValidationError {
                    field: "name".to_string(),
                    message: "Pet name cannot exceed 50 characters".to_string(),
                });
            }
            
            // Check for name conflicts (excluding current pet)
            let name_conflict = pets.values()
                .any(|other_pet| other_pet.id != request.id && 
                     other_pet.name.to_lowercase() == name.to_lowercase());
            
            if name_conflict {
                return Err(PetError::NameConflict {
                    name: name.clone(),
                });
            }
            
            pet.name = name;
        }
        // If new_name is None, we could clear the name, but names are required
        // so we'll ignore None values for the name field
    }
    
    if let Some(new_kind) = request.kind.unfold() {
        if let Some(kind) = new_kind {
            pet.kind = kind;
        }
    }
    
    if let Some(new_age) = request.age.unfold() {
        pet.age = new_age.cloned(); // This handles both Some(age) and None
    }
    
    if let Some(new_behaviors) = request.behaviors.unfold() {
        pet.behaviors = new_behaviors.cloned().unwrap_or_default();
    }
    
    // Update the timestamp
    pet.updated_at = chrono::Utc::now();
    
    Ok(pet.clone())
}
```

## Delete Pet Endpoint

```rust
pub async fn delete_pet(
    state: Arc<AppState>,
    request: DeletePetRequest,
    headers: ApiHeaders,
) -> Result<DeletePetResponse, PetError> {
    // Authenticate the request
    authenticate(&headers)?;
    
    let mut pets = state.pets.lock().unwrap();
    
    match pets.remove(&request.id) {
        Some(deleted_pet) => Ok(DeletePetResponse {
            message: format!("Pet '{}' (ID: {}) deleted successfully", 
                           deleted_pet.name, deleted_pet.id),
            deleted_pet,
        }),
        None => Err(PetError::NotFound {
            pet_id: request.id,
        }),
    }
}
```

## Creating the ReflectAPI Builder

Now let's create the ReflectAPI builder that registers all our endpoints. Create `src/api.rs`:

```rust
use std::sync::Arc;
use crate::{handlers, state::AppState};

pub fn create_api() -> reflectapi::Builder<Arc<AppState>> {
    reflectapi::Builder::new()
        .name("Pet Store API")
        .description("A comprehensive API for managing a pet store")
        .version("1.0.0")
        
        // Health check endpoint
        .route(handlers::health_check, |b| {
            b.name("health.check")
                .readonly(true)
                .tag("health")
                .description("Check the health status of the API")
        })
        
        // Pet management endpoints
        .route(handlers::list_pets, |b| {
            b.name("pets.list")
                .readonly(true)
                .tag("pets")
                .description("List all pets with optional filtering and pagination")
        })
        
        .route(handlers::create_pet, |b| {
            b.name("pets.create")
                .tag("pets")
                .description("Create a new pet in the store")
        })
        
        .route(handlers::get_pet, |b| {
            b.name("pets.get")
                .readonly(true)
                .tag("pets") 
                .description("Get details of a specific pet by ID")
        })
        
        .route(handlers::update_pet, |b| {
            b.name("pets.update")
                .tag("pets")
                .description("Update an existing pet's information")
        })
        
        .route(handlers::delete_pet, |b| {
            b.name("pets.delete")
                .tag("pets")
                .description("Remove a pet from the store")
        })
        
        // Add validation rules
        .validate(|schema| {
            let mut errors = Vec::new();
            
            // Ensure all function names follow our convention
            for function in schema.functions() {
                if !function.name().contains('.') {
                    errors.push(reflectapi::ValidationError::new(
                        reflectapi::ValidationPointer::Function(function.name().to_string()),
                        "Function names should follow 'resource.action' convention".into(),
                    ));
                }
            }
            
            errors
        })
}
```

## Integrating with Axum

Update your `src/main.rs` to create a working web server:

```rust
mod model;
mod api_types;
mod state;
mod handlers;
mod api;

use std::sync::Arc;
use axum::{
    Router,
    extract::State,
    response::Json,
    http::StatusCode,
};
use tower_http::cors::CorsLayer;
use crate::{api::create_api, state::AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::init();
    
    // Create application state
    let app_state = Arc::new(AppState::new());
    
    // Build the ReflectAPI schema
    let api_builder = create_api();
    let schema = api_builder.build()?;
    
    // Create the Axum router with ReflectAPI integration
    let app = Router::new()
        .merge(reflectapi::axum::route(schema, app_state.clone()))
        .layer(CorsLayer::permissive())
        .fallback(not_found_handler);
    
    println!("🚀 Pet Store API starting on http://localhost:3000");
    println!("📖 Health check: http://localhost:3000/health.check");
    println!("🐕 Create pet: POST http://localhost:3000/pets.create");
    println!("📋 List pets: GET http://localhost:3000/pets.list");
    
    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn not_found_handler() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "Endpoint not found",
            "message": "This endpoint doesn't exist in the Pet Store API"
        }))
    )
}
```

## Understanding the Handler Signature

Let's break down why the canonical signature is important:

```rust
async fn handler(
    state: Arc<AppState>,      // Application state (database, config, etc.)
    request: RequestType,      // Strongly-typed request data
    headers: HeadersType,      // HTTP headers (auth, metadata, etc.)
) -> Result<ResponseType, ErrorType>  // Type-safe responses and errors
```

### Benefits of This Pattern

1. **Type Safety**: Request and response types are enforced at compile time
2. **Consistent Client Generation**: All languages get the same method signatures
3. **Error Handling**: HTTP status codes are handled automatically via the `StatusCode` trait
4. **Middleware Integration**: State can carry authentication data from middleware
5. **Documentation**: Types automatically generate OpenAPI schemas

### Why Not Standard Axum Extractors?

```rust
// ❌ This won't work with ReflectAPI
async fn bad_handler(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,  // Breaks client generation!
    Json(request): Json<CreatePetRequest>,
) -> Result<Json<Pet>, AppError>

// ✅ ReflectAPI requires this exact pattern
async fn good_handler(
    state: Arc<AppState>,
    request: CreatePetRequest,
    headers: ApiHeaders,  // Auth data comes through here
) -> Result<Pet, PetError>
```

The ReflectAPI pattern ensures that:
- Generated clients know exactly what parameters to send
- All languages get consistent method signatures
- HTTP metadata is handled predictably
- Middleware data flows through the state parameter

## Testing Your Endpoints

You can now test your API! Start the server:

```bash
cargo run
```

Test the health check:

```bash
curl http://localhost:3000/health.check
```

Create a pet:

```bash
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{
    "name": "Buddy",
    "kind": {
      "type": "dog",
      "breed": "Golden Retriever"
    },
    "age": 3,
    "behaviors": ["Calm"]
  }'
```

List pets:

```bash
curl "http://localhost:3000/pets.list?limit=5" \
  -H "Authorization: Bearer demo-api-key"
```

## What You've Accomplished

✅ **Canonical handler signatures** that enable consistent client generation  
✅ **Authentication middleware** that works with ReflectAPI's constraints  
✅ **Full CRUD operations** for pet management  
✅ **Proper error handling** with HTTP status codes  
✅ **Three-state option handling** for partial updates  
✅ **Working web server** integrated with Axum  

## Understanding Handler Flow

When a request comes in:

1. **Axum** routes the request to the ReflectAPI handler
2. **ReflectAPI** deserializes the request body to your `RequestType`
3. **ReflectAPI** extracts headers and creates your `HeadersType`
4. **Your handler** processes the request with type safety
5. **ReflectAPI** serializes the response and sets the HTTP status code
6. **Generated clients** receive properly typed responses

## Next Steps

Your API endpoints are working! Now let's add comprehensive [validation and error handling](./adding-validation.md) to make your API production-ready.
