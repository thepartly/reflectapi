# Adding Validation

Now that you have working endpoints, let's add basic validation to make your API more robust. With `reflectapi`, validation primarily happens at the type level using standard Serde patterns.

## Basic Input Validation

The most straightforward validation comes from your type definitions. Update your `src/api_types.rs`:

```rust,ignore
use reflectapi::{Input, Output};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input)]
pub struct CreatePetRequest {
    /// Pet's name (1-50 characters)
    pub name: String,
    /// What kind of animal
    pub kind: PetKind,
    /// Age in years (must be reasonable)
    pub age: Option<u8>,
    /// Initial behaviors
    #[serde(default)]
    pub behaviors: Vec<Behavior>,
}
```

## Type-Level Validation

Rust's type system provides automatic validation:

```rust,ignore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Pet {
    pub id: u32,           // Only positive integers
    pub name: String,      // Required field
    pub kind: PetKind,     // Must be valid enum variant
    pub age: Option<u8>,   // 0-255 range automatically enforced
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub behaviors: Vec<Behavior>,
}
```

## Handler Validation

Add validation logic in your handlers. Update `src/handlers.rs`:

```rust,ignore
use crate::api_types::*;
use std::sync::Arc;

pub async fn create_pet(
    state: Arc<AppState>,
    request: CreatePetRequest,
    headers: ApiHeaders,
) -> Result<CreatePetResponse, PetError> {
    // Authenticate
    if headers.authorization != "Bearer demo-api-key" {
        return Err(PetError::Unauthorized);
    }
    
    // Basic validation
    let name = request.name.trim();
    if name.is_empty() {
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
    
    // Age validation
    if let Some(age) = request.age {
        if age > 50 {
            return Err(PetError::ValidationError {
                field: "age".to_string(),
                message: "Pet age seems unrealistic (max 50 years)".to_string(),
            });
        }
    }
    
    // Validate pet kind
    match &request.kind {
        PetKind::Dog { breed } => {
            if breed.trim().is_empty() {
                return Err(PetError::ValidationError {
                    field: "kind.breed".to_string(),
                    message: "Dog breed cannot be empty".to_string(),
                });
            }
        }
        PetKind::Cat { lives } => {
            if *lives == 0 || *lives > 9 {
                return Err(PetError::ValidationError {
                    field: "kind.lives".to_string(),
                    message: "Cats must have 1-9 lives".to_string(),
                });
            }
        }
        PetKind::Bird { wingspan_cm, .. } => {
            if let Some(wingspan) = wingspan_cm {
                if *wingspan == 0 || *wingspan > 500 {
                    return Err(PetError::ValidationError {
                        field: "kind.wingspan_cm".to_string(),
                        message: "Bird wingspan must be 1-500 cm".to_string(),
                    });
                }
            }
        }
    }
    
    // Create the pet
    let pet_id = state.next_pet_id();
    let pet = Pet {
        id: pet_id,
        name: name.to_string(),
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
        message: format!("Pet '{}' created successfully", name),
    })
}
```

## Error Types for Validation

Make sure your error types support validation. In `src/api_types.rs`:

```rust,ignore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Output)]
#[serde(tag = "type")]
pub enum PetError {
    NotFound { id: u32 },
    ValidationError { 
        field: String, 
        message: String 
    },
    Unauthorized,
    DatabaseError { message: String },
}
```

## Testing Your Validation

Test your validation with curl commands:

```bash
# Test with empty name (should fail)
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "", "kind": {"type": "dog", "breed": "Labrador"}}'

# Test with name too long (should fail)
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "This is a very very very very very long pet name that exceeds fifty characters", "kind": {"type": "dog", "breed": "Labrador"}}'

# Test with invalid age (should fail)
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "Buddy", "kind": {"type": "cat", "lives": 5}, "age": 100}'

# Test with valid data (should succeed)
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "Buddy", "kind": {"type": "dog", "breed": "Golden Retriever"}, "age": 3}'
```

## Client-Side Validation

Generated clients automatically handle validation errors:

### TypeScript
```typescript
try {
  const result = await client.pets.create({
    name: "",  // Invalid
    kind: { type: "dog", breed: "Labrador" }
  });
} catch (error) {
  if (error.status === 400) {
    console.log("Validation error:", error.data);
  }
}
```

### Python
```python
try:
    result = await client.pets.create(CreatePetRequest(
        name="",  # Invalid
        kind=PetKind(type="dog", breed="Labrador")
    ))
except Exception as e:
    print(f"Validation error: {e}")
```

## What You've Accomplished

✅ **Type-level validation** using Rust's type system  
✅ **Handler validation** with custom business rules  
✅ **Error responses** with clear validation messages  
✅ **Client compatibility** with generated error handling  

The key insight is that `reflectapi` leverages Rust's type system and standard Serde patterns for validation, rather than requiring additional validation libraries. This keeps your validation logic simple and ensures it works correctly with code generation.

## Next Steps

Your API now has solid validation! Let's move on to [error handling](./error-handling.md) to make your error responses even more consistent and helpful.