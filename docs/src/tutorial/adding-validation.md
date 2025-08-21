# Adding Validation

Now that you have working endpoints, let's add comprehensive validation to make your API production-ready. ReflectAPI provides multiple layers of validation that work seamlessly with client generation.

## Types of Validation in ReflectAPI

1. **Schema-level validation** - Built into your type definitions
2. **Serde validation** - Using serde attributes for serialization rules  
3. **Business logic validation** - Custom validation in your handlers
4. **Schema validation** - Using the ReflectAPI builder's validation system

## Schema-Level Validation

### Using Serde Attributes for Basic Validation

Add validation attributes directly to your types. Update your `src/api_types.rs`:

```rust,ignore
#[derive(serde::Deserialize, Input)]
pub struct CreatePetRequest {
    /// Pet's name (required, 1-50 characters)
    #[serde(deserialize_with = "validate_pet_name")]
    pub name: String,
    /// What kind of animal (required)
    pub kind: PetKind,
    /// Age in years (optional, max 150)
    #[serde(default, deserialize_with = "validate_age")]
    pub age: Option<u8>,
    /// Initial behaviors (optional)
    #[serde(default)]
    pub behaviors: Vec<Behavior>,
}

fn validate_pet_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let name = String::deserialize(deserializer)?;
    
    if name.trim().is_empty() {
        return Err(serde::de::Error::custom("Pet name cannot be empty"));
    }
    
    if name.len() > 50 {
        return Err(serde::de::Error::custom("Pet name cannot exceed 50 characters"));
    }
    
    if name.chars().any(|c| c.is_control()) {
        return Err(serde::de::Error::custom("Pet name cannot contain control characters"));
    }
    
    Ok(name.trim().to_string())
}

fn validate_age<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let age = Option::<u8>::deserialize(deserializer)?;
    
    if let Some(age_value) = age {
        if age_value > 150 {
            return Err(serde::de::Error::custom("Pet age cannot exceed 150 years"));
        }
    }
    
    Ok(age)
}
```

### Basic Validation Patterns

Here's a simplified example showing the validation patterns you just learned:

```rust
/// Demonstrates basic validation patterns for input data
#[derive(Debug, PartialEq)]
pub enum ValidationError {
    Empty,
    TooLong,
    InvalidCharacters,
    TooOld,
}

fn validate_pet_name(name: &str) -> Result<String, ValidationError> {
    let trimmed = name.trim();
    
    if trimmed.is_empty() {
        return Err(ValidationError::Empty);
    }
    
    if trimmed.len() > 50 {
        return Err(ValidationError::TooLong);
    }
    
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(ValidationError::InvalidCharacters);
    }
    
    Ok(trimmed.to_string())
}

fn validate_pet_age(age: Option<u8>) -> Result<Option<u8>, ValidationError> {
    if let Some(age_value) = age {
        if age_value > 150 {
            return Err(ValidationError::TooOld);
        }
    }
    Ok(age)
}

fn main() {
    // Test valid inputs
    assert_eq!(validate_pet_name("Buddy"), Ok("Buddy".to_string()));
    assert_eq!(validate_pet_name("  Max  "), Ok("Max".to_string()));
    assert_eq!(validate_pet_age(Some(5)), Ok(Some(5)));
    assert_eq!(validate_pet_age(None), Ok(None));

    // Test invalid inputs
    assert_eq!(validate_pet_name(""), Err(ValidationError::Empty));
    assert_eq!(validate_pet_name("   "), Err(ValidationError::Empty));

    let long_name = "a".repeat(51);
    assert_eq!(validate_pet_name(&long_name), Err(ValidationError::TooLong));
    assert_eq!(validate_pet_age(Some(151)), Err(ValidationError::TooOld));
}
```

### Using NewType Patterns for Validation

Create validated newtype wrappers. Add to your `src/model.rs`:

```rust,ignore
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(transparent)]
pub struct PetName(String);

impl PetName {
    pub fn new(name: String) -> Result<Self, ValidationError> {
        let trimmed = name.trim();
        
        if trimmed.is_empty() {
            return Err(ValidationError::new("Pet name cannot be empty"));
        }
        
        if trimmed.len() > 50 {
            return Err(ValidationError::new("Pet name cannot exceed 50 characters"));
        }
        
        if trimmed.chars().any(|c| c.is_control()) {
            return Err(ValidationError::new("Pet name cannot contain control characters"));
        }
        
        // Check for offensive content (basic example)
        let lower = trimmed.to_lowercase();
        if lower.contains("bad") || lower.contains("evil") {
            return Err(ValidationError::new("Pet name contains inappropriate content"));
        }
        
        Ok(PetName(trimmed.to_string()))
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PetName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PetName {
    type Err = ValidationError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PetName::new(s.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    message: String,
}

impl ValidationError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(transparent)]
pub struct PetAge(u8);

impl PetAge {
    pub fn new(age: u8) -> Result<Self, ValidationError> {
        if age > 150 {
            return Err(ValidationError::new("Pet age cannot exceed 150 years"));
        }
        
        Ok(PetAge(age))
    }
    
    pub fn value(&self) -> u8 {
        self.0
    }
}
```

## Business Logic Validation

### Comprehensive Handler Validation

Update your handlers with comprehensive validation. Modify `src/handlers.rs`:

```rust,ignore
// Enhanced authentication with detailed error messages
fn authenticate(headers: &ApiHeaders) -> Result<(), PetError> {
    if headers.authorization.is_empty() {
        return Err(PetError::Unauthorized {
            message: "Missing Authorization header. Please provide 'Authorization: Bearer <api-key>'".to_string(),
        });
    }
    
    if !headers.authorization.starts_with("Bearer ") {
        return Err(PetError::Unauthorized {
            message: "Invalid Authorization format. Use 'Authorization: Bearer <api-key>'".to_string(),
        });
    }
    
    let api_key = headers.authorization.strip_prefix("Bearer ").unwrap();
    
    if api_key.is_empty() {
        return Err(PetError::Unauthorized {
            message: "Empty API key provided".to_string(),
        });
    }
    
    if api_key.len() < 8 {
        return Err(PetError::Unauthorized {
            message: "API key too short. Minimum 8 characters required".to_string(),
        });
    }
    
    // In a real app, check against a database or cache
    if !["demo-api-key", "test-key-123"].contains(&api_key) {
        return Err(PetError::Unauthorized {
            message: format!("Invalid API key: '{}'", api_key),
        });
    }
    
    Ok(())
}

// Enhanced pet name validation
fn validate_pet_name_advanced(name: &str, existing_pets: &HashMap<u32, Pet>, exclude_id: Option<u32>) -> Result<(), PetError> {
    // Basic validation
    if name.trim().is_empty() {
        return Err(PetError::ValidationError {
            field: "name".to_string(),
            message: "Pet name cannot be empty".to_string(),
        });
    }
    
    let trimmed_name = name.trim();
    
    if trimmed_name.len() > 50 {
        return Err(PetError::ValidationError {
            field: "name".to_string(),
            message: "Pet name cannot exceed 50 characters".to_string(),
        });
    }
    
    if trimmed_name.len() < 2 {
        return Err(PetError::ValidationError {
            field: "name".to_string(),
            message: "Pet name must be at least 2 characters long".to_string(),
        });
    }
    
    // Character validation
    if trimmed_name.chars().any(|c| c.is_control()) {
        return Err(PetError::ValidationError {
            field: "name".to_string(),
            message: "Pet name cannot contain control characters".to_string(),
        });
    }
    
    if !trimmed_name.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '_') {
        return Err(PetError::ValidationError {
            field: "name".to_string(),
            message: "Pet name can only contain letters, numbers, spaces, hyphens, and underscores".to_string(),
        });
    }
    
    // Check for duplicate names (case-insensitive)
    let name_conflict = existing_pets.values()
        .any(|pet| {
            if let Some(exclude) = exclude_id {
                pet.id != exclude && pet.name.to_lowercase() == trimmed_name.to_lowercase()
            } else {
                pet.name.to_lowercase() == trimmed_name.to_lowercase()
            }
        });
    
    if name_conflict {
        return Err(PetError::NameConflict {
            name: trimmed_name.to_string(),
        });
    }
    
    // Business rule: No pets named after other animals
    let lower_name = trimmed_name.to_lowercase();
    let restricted_names = ["dog", "cat", "bird", "fish", "snake", "hamster", "rabbit"];
    if restricted_names.iter().any(|&restricted| lower_name.contains(restricted)) {
        return Err(PetError::ValidationError {
            field: "name".to_string(),
            message: "Pet name cannot contain other animal names".to_string(),
        });
    }
    
    Ok(())
}

// Validate pet kind specific rules
fn validate_pet_kind(kind: &PetKind) -> Result<(), PetError> {
    match kind {
        PetKind::Dog { breed } => {
            if breed.trim().is_empty() {
                return Err(PetError::ValidationError {
                    field: "kind.breed".to_string(),
                    message: "Dog breed cannot be empty".to_string(),
                });
            }
            
            if breed.len() > 100 {
                return Err(PetError::ValidationError {
                    field: "kind.breed".to_string(),
                    message: "Dog breed name too long (max 100 characters)".to_string(),
                });
            }
        }
        
        PetKind::Cat { lives } => {
            if *lives == 0 {
                return Err(PetError::ValidationError {
                    field: "kind.lives".to_string(),
                    message: "Cats must have at least 1 life remaining".to_string(),
                });
            }
            
            if *lives > 9 {
                return Err(PetError::ValidationError {
                    field: "kind.lives".to_string(),
                    message: "Cats cannot have more than 9 lives".to_string(),
                });
            }
        }
        
        PetKind::Bird { can_talk: _, wingspan_cm } => {
            if let Some(wingspan) = wingspan_cm {
                if *wingspan == 0 {
                    return Err(PetError::ValidationError {
                        field: "kind.wingspan_cm".to_string(),
                        message: "Bird wingspan must be greater than 0".to_string(),
                    });
                }
                
                if *wingspan > 1000 {  // 10 meters seems reasonable as max
                    return Err(PetError::ValidationError {
                        field: "kind.wingspan_cm".to_string(),
                        message: "Bird wingspan too large (max 1000cm)".to_string(),
                    });
                }
            }
        }
    }
    
    Ok(())
}

// Validate behaviors
fn validate_behaviors(behaviors: &[Behavior]) -> Result<(), PetError> {
    if behaviors.len() > 10 {
        return Err(PetError::ValidationError {
            field: "behaviors".to_string(),
            message: "Cannot have more than 10 behaviors per pet".to_string(),
        });
    }
    
    for (i, behavior) in behaviors.iter().enumerate() {
        match behavior {
            Behavior::Aggressive { level, notes } => {
                if *level < 0.0 || *level > 1.0 {
                    return Err(PetError::ValidationError {
                        field: format!("behaviors[{}].level", i),
                        message: "Aggression level must be between 0.0 and 1.0".to_string(),
                    });
                }
                
                if notes.len() > 500 {
                    return Err(PetError::ValidationError {
                        field: format!("behaviors[{}].notes", i),
                        message: "Aggression notes too long (max 500 characters)".to_string(),
                    });
                }
            }
            
            Behavior::Playful { favorite_toy } => {
                if let Some(toy) = favorite_toy {
                    if toy.trim().is_empty() {
                        return Err(PetError::ValidationError {
                            field: format!("behaviors[{}].favorite_toy", i),
                            message: "Favorite toy name cannot be empty".to_string(),
                        });
                    }
                    
                    if toy.len() > 100 {
                        return Err(PetError::ValidationError {
                            field: format!("behaviors[{}].favorite_toy", i),
                            message: "Favorite toy name too long (max 100 characters)".to_string(),
                        });
                    }
                }
            }
            
            Behavior::Other { description } => {
                if description.trim().is_empty() {
                    return Err(PetError::ValidationError {
                        field: format!("behaviors[{}].description", i),
                        message: "Behavior description cannot be empty".to_string(),
                    });
                }
                
                if description.len() > 500 {
                    return Err(PetError::ValidationError {
                        field: format!("behaviors[{}].description", i),
                        message: "Behavior description too long (max 500 characters)".to_string(),
                    });
                }
            }
            
            Behavior::Calm => {
                // No additional validation needed for Calm
            }
        }
    }
    
    Ok(())
}
```

### Updated Create Pet Handler with Full Validation

```rust,ignore
pub async fn create_pet(
    state: Arc<AppState>,
    request: CreatePetRequest,
    headers: ApiHeaders,
) -> Result<CreatePetResponse, PetError> {
    // Authenticate the request
    authenticate(&headers)?;
    
    // Get a read lock first to check for name conflicts
    let pets = state.pets.lock().unwrap();
    
    // Validate pet name with conflict checking
    validate_pet_name_advanced(&request.name, &pets, None)?;
    
    // Validate pet kind
    validate_pet_kind(&request.kind)?;
    
    // Validate age if provided
    if let Some(age) = request.age {
        if age > 150 {
            return Err(PetError::ValidationError {
                field: "age".to_string(),
                message: "Pet age cannot exceed 150 years".to_string(),
            });
        }
    }
    
    // Validate behaviors
    validate_behaviors(&request.behaviors)?;
    
    // Business rule: Aggressive pets must have age specified
    let has_aggressive_behavior = request.behaviors.iter()
        .any(|b| matches!(b, Behavior::Aggressive { .. }));
    
    if has_aggressive_behavior && request.age.is_none() {
        return Err(PetError::ValidationError {
            field: "age".to_string(),
            message: "Age is required for pets with aggressive behavior".to_string(),
        });
    }
    
    // Business rule: Very young pets cannot be aggressive
    if let Some(age) = request.age {
        if age < 1 && has_aggressive_behavior {
            return Err(PetError::ValidationError {
                field: "behaviors".to_string(),
                message: "Pets under 1 year old cannot have aggressive behavior".to_string(),
            });
        }
    }
    
    drop(pets); // Release the read lock
    
    // Create the new pet
    let pet_id = state.next_pet_id();
    let pet = Pet {
        id: pet_id,
        name: request.name.trim().to_string(),
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
        message: format!("Pet '{}' created successfully with ID {}", pet.name, pet_id),
    })
}
```

## Schema-Level Validation with Builder

Add validation rules to your API builder. Update `src/api.rs`:

```rust,ignore
pub fn create_api() -> reflectapi::Builder<Arc<AppState>> {
    reflectapi::Builder::new()
        .name("Pet Store API")
        .description("A comprehensive API for managing a pet store")
        .version("1.0.0")
        
        // ... your existing routes ...
        
        // Comprehensive validation rules
        .validate(|schema| {
            let mut errors = Vec::new();
            
            // Function naming convention
            for function in schema.functions() {
                let name = function.name();
                
                if !name.contains('.') {
                    errors.push(reflectapi::ValidationError::new(
                        reflectapi::ValidationPointer::Function(name.to_string()),
                        "Function names should follow 'resource.action' convention".into(),
                    ));
                }
                
                let parts: Vec<&str> = name.split('.').collect();
                if parts.len() != 2 {
                    errors.push(reflectapi::ValidationError::new(
                        reflectapi::ValidationPointer::Function(name.to_string()),
                        "Function names should have exactly one dot (e.g., 'pets.create')".into(),
                    ));
                }
                
                // Validate resource names
                if let Some(resource) = parts.first() {
                    if resource.is_empty() || !resource.chars().all(|c| c.is_ascii_alphanumeric()) {
                        errors.push(reflectapi::ValidationError::new(
                            reflectapi::ValidationPointer::Function(name.to_string()),
                            "Resource names should only contain alphanumeric characters".into(),
                        ));
                    }
                }
                
                // Validate action names
                if let Some(action) = parts.get(1) {
                    let valid_actions = ["list", "create", "get", "update", "delete", "check"];
                    if !valid_actions.contains(action) {
                        errors.push(reflectapi::ValidationError::new(
                            reflectapi::ValidationPointer::Function(name.to_string()),
                            format!("Action '{}' not in allowed list: {:?}", action, valid_actions).into(),
                        ));
                    }
                }
            }
            
            // Type naming validation
            for type_def in schema.types() {
                let type_name = type_def.name();
                
                // Ensure types follow PascalCase
                if !type_name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    errors.push(reflectapi::ValidationError::new(
                        reflectapi::ValidationPointer::Type(type_name.to_string()),
                        "Type names should start with uppercase letter (PascalCase)".into(),
                    ));
                }
                
                // Ensure no type names conflict with reserved words
                let reserved_words = ["Error", "Request", "Response", "String", "Number", "Boolean"];
                if reserved_words.contains(&type_name) {
                    errors.push(reflectapi::ValidationError::new(
                        reflectapi::ValidationPointer::Type(type_name.to_string()),
                        format!("Type name '{}' conflicts with reserved word", type_name).into(),
                    ));
                }
            }
            
            errors
        })
}
```

## Input Sanitization

Add input sanitization helpers in `src/handlers.rs`:

```rust,ignore
// Sanitization helpers
fn sanitize_text_input(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect::<String>()
        .replace(char::is_control, "")
}

fn sanitize_search_input(input: &str) -> String {
    let sanitized = sanitize_text_input(input);
    // Remove potentially dangerous characters for search
    sanitized
        .replace(['%', '_', '*', '?'], "")
        .chars()
        .take(100) // Limit search terms
        .collect()
}

// Updated list_pets with input sanitization
pub async fn list_pets(
    state: Arc<AppState>,
    request: ListPetsRequest,
    headers: ApiHeaders,
) -> Result<PaginatedPets, PetError> {
    authenticate(&headers)?;
    
    // Validate pagination parameters
    if let Some(limit) = request.limit {
        if limit > 100 {
            return Err(PetError::ValidationError {
                field: "limit".to_string(),
                message: "Limit cannot exceed 100 items per page".to_string(),
            });
        }
    }
    
    // Sanitize and validate cursor
    let cursor_offset = if let Some(cursor) = &request.cursor {
        let sanitized_cursor = sanitize_text_input(cursor);
        sanitized_cursor.parse::<usize>().map_err(|_| PetError::ValidationError {
            field: "cursor".to_string(),
            message: "Invalid cursor format - must be a number".to_string(),
        })?
    } else {
        0
    };
    
    // Validate and sanitize kind filter
    let kind_filter = if let Some(filter) = &request.kind_filter {
        let sanitized = sanitize_search_input(filter).to_lowercase();
        match sanitized.as_str() {
            "dog" | "cat" | "bird" => Some(sanitized),
            "" => None,
            _ => return Err(PetError::ValidationError {
                field: "kind_filter".to_string(),
                message: "Invalid kind filter. Must be 'dog', 'cat', or 'bird'".to_string(),
            }),
        }
    } else {
        None
    };
    
    // ... rest of the function logic ...
}
```

## Rate Limiting and Security

Add basic rate limiting in your handlers:

```rust,ignore
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Simple in-memory rate limiter (use Redis in production)
#[derive(Debug)]
struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Mutex::new(HashMap::new()),
            max_requests,
            window,
        }
    }
    
    fn is_allowed(&self, key: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Instant::now();
        
        let user_requests = requests.entry(key.to_string()).or_insert_with(Vec::new);
        
        // Remove old requests outside the window
        user_requests.retain(|&time| now.duration_since(time) < self.window);
        
        // Check if under limit
        if user_requests.len() < self.max_requests {
            user_requests.push(now);
            true
        } else {
            false
        }
    }
}

// Add rate limiter to AppState
impl AppState {
    pub fn new() -> Self {
        Self {
            pets: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
            api_keys: vec!["demo-api-key".to_string(), "test-key-123".to_string()],
            rate_limiter: RateLimiter::new(100, Duration::from_secs(60)), // 100 requests per minute
        }
    }
}

// Enhanced authentication with rate limiting
fn authenticate_with_rate_limiting(headers: &ApiHeaders, state: &AppState) -> Result<(), PetError> {
    // Extract API key
    if headers.authorization.is_empty() {
        return Err(PetError::Unauthorized {
            message: "Missing Authorization header".to_string(),
        });
    }
    
    let api_key = headers.authorization.strip_prefix("Bearer ")
        .ok_or_else(|| PetError::Unauthorized {
            message: "Invalid Authorization format".to_string(),
        })?;
    
    // Rate limiting
    if !state.rate_limiter.is_allowed(api_key) {
        return Err(PetError::ValidationError {
            field: "rate_limit".to_string(),
            message: "Rate limit exceeded. Please try again later".to_string(),
        });
    }
    
    // Authenticate
    if !state.api_keys.contains(&api_key.to_string()) {
        return Err(PetError::Unauthorized {
            message: "Invalid API key".to_string(),
        });
    }
    
    Ok(())
}
```

## Error Context and Logging

Add structured error reporting:

```rust,ignore
use tracing::{info, warn, error};

// Enhanced error context
#[derive(serde::Serialize, Output)]
pub struct DetailedValidationError {
    pub error_type: String,
    pub field: String,
    pub message: String,
    pub provided_value: Option<String>,
    pub constraints: ValidationConstraints,
}

#[derive(serde::Serialize, Output)]
pub struct ValidationConstraints {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<String>>,
}

// Logging middleware function
fn log_validation_error(field: &str, message: &str, value: Option<&str>) {
    warn!(
        field = field,
        message = message,
        value = value,
        "Validation failed for field"
    );
}

// Example usage in create_pet
pub async fn create_pet_with_logging(
    state: Arc<AppState>,
    request: CreatePetRequest,
    headers: ApiHeaders,
) -> Result<CreatePetResponse, PetError> {
    info!("Creating new pet: {}", request.name);
    
    match authenticate_with_rate_limiting(&headers, &state) {
        Ok(_) => info!("Authentication successful"),
        Err(e) => {
            warn!("Authentication failed: {:?}", e);
            return Err(e);
        }
    }
    
    // ... validation with logging ...
    
    info!("Pet '{}' created successfully", request.name);
    // ... rest of function
}
```

## Testing Your Validation

Test your validation with curl:

```bash
# Test empty name validation
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "", "kind": {"type": "dog", "breed": "Test"}}'

# Test name too long
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "This is a very long name that exceeds the fifty character limit", "kind": {"type": "dog", "breed": "Test"}}'

# Test invalid age
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "TestPet", "kind": {"type": "cat", "lives": 5}, "age": 200}'

# Test rate limiting (run this many times quickly)
for i in {1..110}; do
  curl -X POST http://localhost:3000/pets.create \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer demo-api-key" \
    -d '{"name": "Pet'$i'", "kind": {"type": "dog", "breed": "Test"}}'
done
```

## What You've Accomplished

✅ **Schema-level validation** with serde attributes and custom deserializers  
✅ **NewType patterns** for type-safe validation  
✅ **Business logic validation** with comprehensive rules  
✅ **Input sanitization** to prevent injection attacks  
✅ **Rate limiting** to prevent abuse  
✅ **Error context** with detailed validation messages  
✅ **Structured logging** for debugging and monitoring  

## Benefits of ReflectAPI's Validation

1. **Type Safety**: Validation errors are compile-time checked types
2. **Client Generation**: Validation constraints appear in generated client documentation
3. **Consistent Errors**: HTTP status codes are handled automatically
4. **Developer Experience**: Clear, actionable error messages
5. **Security**: Built-in protection against common attacks

## Next Steps

Your API now has comprehensive validation! Let's move on to [error handling](./error-handling.md) to make your error responses even more helpful and consistent.
