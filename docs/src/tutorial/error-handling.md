# Error Handling

Error handling is crucial for creating reliable APIs. `reflectapi` provides powerful tools for creating consistent, informative error responses that work seamlessly across all generated clients.

## `reflectapi`'s Error Handling Philosophy

`reflectapi` treats errors as first-class citizens:

1. **Type-safe errors** - Errors are strongly typed with derive macros
2. **HTTP status codes** - Automatic mapping via the `StatusCode` trait
3. **Client generation** - Error types become typed exceptions/errors in clients
4. **Consistent structure** - All languages get the same error format

## Understanding the StatusCode Trait

The `StatusCode` trait maps your error types to HTTP status codes:

```rust,ignore
use reflectapi::{Output, StatusCode};

#[derive(serde::Serialize, Output)]
pub enum PetError {
    NotFound { pet_id: u32 },
    NameConflict { name: String },
    ValidationError { field: String, message: String },
    Unauthorized { message: String },
    InternalError { message: String },
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
```

## Comprehensive Error Hierarchy

Let's create a robust error system. Update your `src/api_types.rs`:

```rust,ignore
use reflectapi::{Output, StatusCode};
use std::fmt;

// ============================================================================
// Base Error Traits
// ============================================================================

pub trait ApiError: std::error::Error + Send + Sync {
    fn error_code(&self) -> &'static str;
    fn user_message(&self) -> String;
    fn developer_message(&self) -> Option<String> { None }
    fn error_id(&self) -> Option<String> { None }
}

// ============================================================================
// Authentication Errors
// ============================================================================

#[derive(Debug, serde::Serialize, Output)]
#[serde(tag = "error_type")]
pub enum AuthError {
    MissingCredentials {
        message: String,
        required_format: String,
    },
    InvalidCredentials {
        message: String,
        provided_key_prefix: String,
    },
    ExpiredCredentials {
        message: String,
        expires_at: chrono::DateTime<chrono::Utc>,
    },
    InsufficientPermissions {
        message: String,
        required_permission: String,
        user_permissions: Vec<String>,
    },
}

impl StatusCode for AuthError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            AuthError::MissingCredentials { .. } => http::StatusCode::UNAUTHORIZED,
            AuthError::InvalidCredentials { .. } => http::StatusCode::UNAUTHORIZED,
            AuthError::ExpiredCredentials { .. } => http::StatusCode::UNAUTHORIZED,
            AuthError::InsufficientPermissions { .. } => http::StatusCode::FORBIDDEN,
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::MissingCredentials { message, .. } => write!(f, "Missing credentials: {}", message),
            AuthError::InvalidCredentials { message, .. } => write!(f, "Invalid credentials: {}", message),
            AuthError::ExpiredCredentials { message, .. } => write!(f, "Expired credentials: {}", message),
            AuthError::InsufficientPermissions { message, .. } => write!(f, "Insufficient permissions: {}", message),
        }
    }
}

impl std::error::Error for AuthError {}

impl ApiError for AuthError {
    fn error_code(&self) -> &'static str {
        match self {
            AuthError::MissingCredentials { .. } => "AUTH_MISSING_CREDENTIALS",
            AuthError::InvalidCredentials { .. } => "AUTH_INVALID_CREDENTIALS", 
            AuthError::ExpiredCredentials { .. } => "AUTH_EXPIRED_CREDENTIALS",
            AuthError::InsufficientPermissions { .. } => "AUTH_INSUFFICIENT_PERMISSIONS",
        }
    }
    
    fn user_message(&self) -> String {
        match self {
            AuthError::MissingCredentials { .. } => "Authentication required".to_string(),
            AuthError::InvalidCredentials { .. } => "Invalid credentials provided".to_string(),
            AuthError::ExpiredCredentials { .. } => "Your session has expired".to_string(),
            AuthError::InsufficientPermissions { .. } => "You don't have permission to perform this action".to_string(),
        }
    }
    
    fn developer_message(&self) -> Option<String> {
        Some(self.to_string())
    }
}

// ============================================================================
// Validation Errors
// ============================================================================

#[derive(Debug, serde::Serialize, Output)]
pub struct ValidationError {
    pub field: String,
    pub code: ValidationErrorCode,
    pub message: String,
    pub provided_value: Option<serde_json::Value>,
    pub constraints: ValidationConstraints,
}

#[derive(Debug, serde::Serialize, Output)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationErrorCode {
    Required,
    TooShort,
    TooLong,
    InvalidFormat,
    OutOfRange,
    InvalidEnum,
    Duplicate,
    Forbidden,
    Custom,
}

#[derive(Debug, serde::Serialize, Output)]
pub struct ValidationConstraints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<String>>,
}

impl ValidationError {
    pub fn required(field: &str) -> Self {
        Self {
            field: field.to_string(),
            code: ValidationErrorCode::Required,
            message: format!("Field '{}' is required", field),
            provided_value: None,
            constraints: ValidationConstraints {
                min_length: None,
                max_length: None,
                min_value: None,
                max_value: None,
                pattern: None,
                allowed_values: None,
            },
        }
    }
    
    pub fn too_short(field: &str, value: &str, min_length: usize) -> Self {
        Self {
            field: field.to_string(),
            code: ValidationErrorCode::TooShort,
            message: format!("Field '{}' must be at least {} characters long", field, min_length),
            provided_value: Some(serde_json::Value::String(value.to_string())),
            constraints: ValidationConstraints {
                min_length: Some(min_length),
                max_length: None,
                min_value: None,
                max_value: None,
                pattern: None,
                allowed_values: None,
            },
        }
    }
    
    pub fn too_long(field: &str, value: &str, max_length: usize) -> Self {
        Self {
            field: field.to_string(),
            code: ValidationErrorCode::TooLong,
            message: format!("Field '{}' cannot exceed {} characters", field, max_length),
            provided_value: Some(serde_json::Value::String(value.to_string())),
            constraints: ValidationConstraints {
                min_length: None,
                max_length: Some(max_length),
                min_value: None,
                max_value: None,
                pattern: None,
                allowed_values: None,
            },
        }
    }
    
    pub fn invalid_format(field: &str, value: &str, pattern: &str) -> Self {
        Self {
            field: field.to_string(),
            code: ValidationErrorCode::InvalidFormat,
            message: format!("Field '{}' has invalid format", field),
            provided_value: Some(serde_json::Value::String(value.to_string())),
            constraints: ValidationConstraints {
                min_length: None,
                max_length: None,
                min_value: None,
                max_value: None,
                pattern: Some(pattern.to_string()),
                allowed_values: None,
            },
        }
    }
    
    pub fn duplicate(field: &str, value: &str) -> Self {
        Self {
            field: field.to_string(),
            code: ValidationErrorCode::Duplicate,
            message: format!("Value '{}' already exists for field '{}'", value, field),
            provided_value: Some(serde_json::Value::String(value.to_string())),
            constraints: ValidationConstraints {
                min_length: None,
                max_length: None,
                min_value: None,
                max_value: None,
                pattern: None,
                allowed_values: None,
            },
        }
    }
}

#[derive(Debug, serde::Serialize, Output)]
pub struct ValidationErrors {
    pub errors: Vec<ValidationError>,
    pub error_count: usize,
}

impl ValidationErrors {
    pub fn new(errors: Vec<ValidationError>) -> Self {
        let error_count = errors.len();
        Self { errors, error_count }
    }
    
    pub fn single(error: ValidationError) -> Self {
        Self::new(vec![error])
    }
    
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.error_count = self.errors.len();
    }
    
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl StatusCode for ValidationErrors {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::BAD_REQUEST
    }
}

// ============================================================================
// Business Logic Errors
// ============================================================================

#[derive(Debug, serde::Serialize, Output)]
#[serde(tag = "error_type")]
pub enum BusinessError {
    ResourceNotFound {
        resource_type: String,
        resource_id: String,
        message: String,
    },
    ResourceConflict {
        resource_type: String,
        conflict_field: String,
        conflict_value: String,
        message: String,
    },
    BusinessRuleViolation {
        rule_name: String,
        message: String,
        details: serde_json::Value,
    },
    RateLimitExceeded {
        limit: u32,
        window_seconds: u32,
        retry_after_seconds: u32,
        message: String,
    },
}

impl StatusCode for BusinessError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            BusinessError::ResourceNotFound { .. } => http::StatusCode::NOT_FOUND,
            BusinessError::ResourceConflict { .. } => http::StatusCode::CONFLICT,
            BusinessError::BusinessRuleViolation { .. } => http::StatusCode::UNPROCESSABLE_ENTITY,
            BusinessError::RateLimitExceeded { .. } => http::StatusCode::TOO_MANY_REQUESTS,
        }
    }
}

// ============================================================================
// Comprehensive Pet Store Error
// ============================================================================

#[derive(Debug, serde::Serialize, Output)]
#[serde(tag = "category", content = "details")]
pub enum PetStoreError {
    Authentication(AuthError),
    Validation(ValidationErrors), 
    Business(BusinessError),
    Internal(InternalError),
}

impl StatusCode for PetStoreError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            PetStoreError::Authentication(e) => e.status_code(),
            PetStoreError::Validation(e) => e.status_code(),
            PetStoreError::Business(e) => e.status_code(),
            PetStoreError::Internal(e) => e.status_code(),
        }
    }
}

impl From<AuthError> for PetStoreError {
    fn from(error: AuthError) -> Self {
        PetStoreError::Authentication(error)
    }
}

impl From<ValidationErrors> for PetStoreError {
    fn from(error: ValidationErrors) -> Self {
        PetStoreError::Validation(error)
    }
}

impl From<ValidationError> for PetStoreError {
    fn from(error: ValidationError) -> Self {
        PetStoreError::Validation(ValidationErrors::single(error))
    }
}

impl From<BusinessError> for PetStoreError {
    fn from(error: BusinessError) -> Self {
        PetStoreError::Business(error)
    }
}

#[derive(Debug, serde::Serialize, Output)]
pub struct InternalError {
    pub error_id: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl InternalError {
    pub fn new(message: &str) -> Self {
        Self {
            error_id: uuid::Uuid::new_v4().to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

impl StatusCode for InternalError {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl From<InternalError> for PetStoreError {
    fn from(error: InternalError) -> Self {
        PetStoreError::Internal(error)
    }
}
```

## Advanced Error Handlers

Create helper functions for common error patterns. Add to `src/handlers.rs`:

```rust,ignore
use tracing::{error, warn, info};
use crate::api_types::*;

// ============================================================================
// Error Construction Helpers  
// ============================================================================

pub fn not_found_error(resource_type: &str, id: u32) -> PetStoreError {
    BusinessError::ResourceNotFound {
        resource_type: resource_type.to_string(),
        resource_id: id.to_string(),
        message: format!("{} with ID {} not found", resource_type, id),
    }.into()
}

pub fn conflict_error(resource_type: &str, field: &str, value: &str) -> PetStoreError {
    BusinessError::ResourceConflict {
        resource_type: resource_type.to_string(),
        conflict_field: field.to_string(),
        conflict_value: value.to_string(),
        message: format!("{} with {} '{}' already exists", resource_type, field, value),
    }.into()
}

pub fn validation_error_from_field(field: &str, message: &str) -> PetStoreError {
    ValidationError {
        field: field.to_string(),
        code: ValidationErrorCode::Custom,
        message: message.to_string(),
        provided_value: None,
        constraints: ValidationConstraints {
            min_length: None,
            max_length: None,
            min_value: None,
            max_value: None,
            pattern: None,
            allowed_values: None,
        },
    }.into()
}

pub fn rate_limit_error(limit: u32, window_seconds: u32) -> PetStoreError {
    BusinessError::RateLimitExceeded {
        limit,
        window_seconds,
        retry_after_seconds: window_seconds,
        message: format!("Rate limit of {} requests per {} seconds exceeded", limit, window_seconds),
    }.into()
}

// ============================================================================
// Error Logging and Monitoring
// ============================================================================

pub fn log_error(error: &PetStoreError, context: &str) {
    match error {
        PetStoreError::Authentication(auth_err) => {
            warn!(
                error_type = "authentication",
                context = context,
                error = ?auth_err,
                "Authentication error occurred"
            );
        }
        PetStoreError::Validation(val_errs) => {
            info!(
                error_type = "validation", 
                context = context,
                error_count = val_errs.error_count,
                "Validation errors occurred"
            );
            
            for val_err in &val_errs.errors {
                info!(
                    field = val_err.field,
                    code = ?val_err.code,
                    message = val_err.message,
                    "Validation error detail"
                );
            }
        }
        PetStoreError::Business(bus_err) => {
            warn!(
                error_type = "business",
                context = context,
                error = ?bus_err,
                "Business logic error occurred"
            );
        }
        PetStoreError::Internal(int_err) => {
            error!(
                error_type = "internal",
                context = context,
                error_id = int_err.error_id,
                message = int_err.message,
                "Internal error occurred"
            );
        }
    }
}

// ============================================================================
// Enhanced Authentication
// ============================================================================

pub fn authenticate_request(headers: &ApiHeaders) -> Result<(), PetStoreError> {
    if headers.authorization.is_empty() {
        return Err(AuthError::MissingCredentials {
            message: "Authorization header is required".to_string(),
            required_format: "Bearer <api-key>".to_string(),
        }.into());
    }
    
    if !headers.authorization.starts_with("Bearer ") {
        return Err(AuthError::InvalidCredentials {
            message: "Authorization header must use Bearer token format".to_string(),
            provided_key_prefix: headers.authorization.chars().take(10).collect(),
        }.into());
    }
    
    let api_key = headers.authorization.strip_prefix("Bearer ").unwrap();
    
    if api_key.is_empty() {
        return Err(AuthError::InvalidCredentials {
            message: "API key cannot be empty".to_string(),
            provided_key_prefix: "".to_string(),
        }.into());
    }
    
    if api_key.len() < 8 {
        return Err(AuthError::InvalidCredentials {
            message: "API key too short".to_string(),
            provided_key_prefix: api_key.chars().take(3).collect(),
        }.into());
    }
    
    // In production, check against database/cache
    let valid_keys = ["demo-api-key", "test-key-123"];
    if !valid_keys.contains(&api_key) {
        return Err(AuthError::InvalidCredentials {
            message: "Invalid API key".to_string(),
            provided_key_prefix: api_key.chars().take(8).collect(),
        }.into());
    }
    
    Ok(())
}

// ============================================================================
// Comprehensive Validation
// ============================================================================

pub struct ValidationContext {
    errors: Vec<ValidationError>,
}

impl ValidationContext {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }
    
    pub fn validate_pet_name(&mut self, name: &str, existing_pets: &HashMap<u32, Pet>, exclude_id: Option<u32>) {
        let trimmed = name.trim();
        
        if trimmed.is_empty() {
            self.errors.push(ValidationError::required("name"));
            return;
        }
        
        if trimmed.len() < 2 {
            self.errors.push(ValidationError::too_short("name", trimmed, 2));
        }
        
        if trimmed.len() > 50 {
            self.errors.push(ValidationError::too_long("name", trimmed, 50));
        }
        
        if trimmed.chars().any(|c| c.is_control()) {
            self.errors.push(ValidationError::invalid_format("name", trimmed, "No control characters allowed"));
        }
        
        if !trimmed.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '_') {
            self.errors.push(ValidationError::invalid_format("name", trimmed, "Only letters, numbers, spaces, hyphens, and underscores allowed"));
        }
        
        // Check for duplicates
        let name_exists = existing_pets.values().any(|pet| {
            if let Some(exclude) = exclude_id {
                pet.id != exclude && pet.name.to_lowercase() == trimmed.to_lowercase()
            } else {
                pet.name.to_lowercase() == trimmed.to_lowercase()
            }
        });
        
        if name_exists {
            self.errors.push(ValidationError::duplicate("name", trimmed));
        }
    }
    
    pub fn validate_pet_age(&mut self, age: Option<u8>) {
        if let Some(age_value) = age {
            if age_value > 150 {
                self.errors.push(ValidationError {
                    field: "age".to_string(),
                    code: ValidationErrorCode::OutOfRange,
                    message: "Pet age cannot exceed 150 years".to_string(),
                    provided_value: Some(serde_json::Value::Number(age_value.into())),
                    constraints: ValidationConstraints {
                        min_length: None,
                        max_length: None,
                        min_value: Some(0.0),
                        max_value: Some(150.0),
                        pattern: None,
                        allowed_values: None,
                    },
                });
            }
        }
    }
    
    pub fn validate_pet_kind(&mut self, kind: &PetKind) {
        match kind {
            PetKind::Dog { breed } => {
                if breed.trim().is_empty() {
                    self.errors.push(ValidationError::required("kind.breed"));
                } else if breed.len() > 100 {
                    self.errors.push(ValidationError::too_long("kind.breed", breed, 100));
                }
            }
            PetKind::Cat { lives } => {
                if *lives == 0 {
                    self.errors.push(ValidationError {
                        field: "kind.lives".to_string(),
                        code: ValidationErrorCode::OutOfRange,
                        message: "Cats must have at least 1 life".to_string(),
                        provided_value: Some(serde_json::Value::Number((*lives).into())),
                        constraints: ValidationConstraints {
                            min_length: None,
                            max_length: None,
                            min_value: Some(1.0),
                            max_value: Some(9.0),
                            pattern: None,
                            allowed_values: None,
                        },
                    });
                } else if *lives > 9 {
                    self.errors.push(ValidationError {
                        field: "kind.lives".to_string(),
                        code: ValidationErrorCode::OutOfRange,
                        message: "Cats cannot have more than 9 lives".to_string(),
                        provided_value: Some(serde_json::Value::Number((*lives).into())),
                        constraints: ValidationConstraints {
                            min_length: None,
                            max_length: None,
                            min_value: Some(1.0),
                            max_value: Some(9.0),
                            pattern: None,
                            allowed_values: None,
                        },
                    });
                }
            }
            PetKind::Bird { wingspan_cm, .. } => {
                if let Some(wingspan) = wingspan_cm {
                    if *wingspan == 0 {
                        self.errors.push(ValidationError {
                            field: "kind.wingspan_cm".to_string(),
                            code: ValidationErrorCode::OutOfRange,
                            message: "Bird wingspan must be greater than 0".to_string(),
                            provided_value: Some(serde_json::Value::Number((*wingspan).into())),
                            constraints: ValidationConstraints {
                                min_length: None,
                                max_length: None,
                                min_value: Some(1.0),
                                max_value: Some(1000.0),
                                pattern: None,
                                allowed_values: None,
                            },
                        });
                    } else if *wingspan > 1000 {
                        self.errors.push(ValidationError {
                            field: "kind.wingspan_cm".to_string(),
                            code: ValidationErrorCode::OutOfRange,
                            message: "Bird wingspan too large (max 1000cm)".to_string(),
                            provided_value: Some(serde_json::Value::Number((*wingspan).into())),
                            constraints: ValidationConstraints {
                                min_length: None,
                                max_length: None,
                                min_value: Some(1.0),
                                max_value: Some(1000.0),
                                pattern: None,
                                allowed_values: None,
                            },
                        });
                    }
                }
            }
        }
    }
    
    pub fn finish(self) -> Result<(), PetStoreError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors::new(self.errors).into())
        }
    }
}
```

## Updated Create Pet Handler with Advanced Error Handling

```rust,ignore
pub async fn create_pet(
    state: Arc<AppState>,
    request: CreatePetRequest,
    headers: ApiHeaders,
) -> Result<CreatePetResponse, PetStoreError> {
    let operation_context = "create_pet";
    
    // Authentication
    if let Err(auth_error) = authenticate_request(&headers) {
        log_error(&auth_error, operation_context);
        return Err(auth_error);
    }
    
    // Comprehensive validation
    let pets = state.pets.lock().unwrap();
    let mut validation_ctx = ValidationContext::new();
    
    validation_ctx.validate_pet_name(&request.name, &pets, None);
    validation_ctx.validate_pet_age(request.age);
    validation_ctx.validate_pet_kind(&request.kind);
    
    // Business rule validation
    let has_aggressive_behavior = request.behaviors.iter()
        .any(|b| matches!(b, Behavior::Aggressive { .. }));
    
    if has_aggressive_behavior && request.age.is_none() {
        validation_ctx.errors.push(ValidationError {
            field: "age".to_string(),
            code: ValidationErrorCode::Required,
            message: "Age is required for pets with aggressive behavior".to_string(),
            provided_value: None,
            constraints: ValidationConstraints {
                min_length: None,
                max_length: None,
                min_value: None,
                max_value: None,
                pattern: None,
                allowed_values: None,
            },
        });
    }
    
    if let Some(age) = request.age {
        if age < 1 && has_aggressive_behavior {
            validation_ctx.errors.push(ValidationError {
                field: "behaviors".to_string(),
                code: ValidationErrorCode::BusinessRuleViolation,
                message: "Pets under 1 year old cannot have aggressive behavior".to_string(),
                provided_value: Some(serde_json::json!({
                    "age": age,
                    "has_aggressive": true
                })),
                constraints: ValidationConstraints {
                    min_length: None,
                    max_length: None,
                    min_value: Some(1.0),
                    max_value: None,
                    pattern: None,
                    allowed_values: None,
                },
            });
        }
    }
    
    // Check validation results
    if let Err(validation_error) = validation_ctx.finish() {
        log_error(&validation_error, operation_context);
        return Err(validation_error);
    }
    
    drop(pets); // Release read lock
    
    // Create the pet
    let pet_id = state.next_pet_id();
    let pet = Pet {
        id: pet_id,
        name: request.name.trim().to_string(),
        kind: request.kind,
        age: request.age,
        updated_at: chrono::Utc::now(),
        behaviors: request.behaviors,
    };
    
    // Store with error handling
    match state.pets.lock() {
        Ok(mut pets) => {
            pets.insert(pet_id, pet.clone());
            info!(
                pet_id = pet_id,
                pet_name = pet.name,
                "Pet created successfully"
            );
            
            Ok(CreatePetResponse {
                pet,
                message: format!("Pet '{}' created successfully with ID {}", pet.name, pet_id),
            })
        }
        Err(_) => {
            let internal_error = InternalError::new("Failed to acquire lock on pet storage");
            log_error(&internal_error.clone().into(), operation_context);
            Err(internal_error.into())
        }
    }
}
```

## Error Recovery and Resilience

Add retry logic and graceful degradation:

```rust,ignore
use std::time::Duration;
use tokio::time::sleep;

pub async fn get_pet_with_retry(
    state: Arc<AppState>,
    request: GetPetRequest,
    headers: ApiHeaders,
) -> Result<Pet, PetStoreError> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_millis(100);
    
    // Authentication (don't retry auth failures)
    authenticate_request(&headers)?;
    
    // Retry logic for storage access
    for attempt in 1..=MAX_RETRIES {
        match state.pets.lock() {
            Ok(pets) => {
                return match pets.get(&request.id) {
                    Some(pet) => {
                        info!(pet_id = request.id, "Pet retrieved successfully");
                        Ok(pet.clone())
                    }
                    None => {
                        let error = not_found_error("Pet", request.id);
                        log_error(&error, "get_pet");
                        Err(error)
                    }
                };
            }
            Err(_) if attempt < MAX_RETRIES => {
                warn!(
                    attempt = attempt,
                    max_retries = MAX_RETRIES,
                    "Failed to acquire lock, retrying"
                );
                sleep(RETRY_DELAY * attempt).await;
                continue;
            }
            Err(_) => {
                let error = InternalError::new("Failed to acquire lock after retries");
                log_error(&error.clone().into(), "get_pet");
                return Err(error.into());
            }
        }
    }
    
    unreachable!()
}
```

## Client-Side Error Handling

### TypeScript Client
Generated TypeScript clients get strongly typed errors:

```typescript
try {
  const pet = await client.pets.create({
    name: "Buddy",
    kind: { type: "dog", breed: "Golden Retriever" }
  });
  console.log("Pet created:", pet);
} catch (error) {
  if (error.status === 400) {
    // Validation errors
    const validationError = error.data as ValidationErrors;
    validationError.errors.forEach(err => {
      console.error(`Validation error in ${err.field}: ${err.message}`);
    });
  } else if (error.status === 401) {
    // Authentication errors
    console.error("Authentication failed:", error.data);
  } else if (error.status === 409) {
    // Conflict errors
    console.error("Resource conflict:", error.data);
  }
}
```

### Python Client
Generated Python clients use exceptions:

```python
try:
    pet = await client.pets.create(CreatePetRequest(
        name="Buddy",
        kind=PetKind(type="dog", breed="Golden Retriever")
    ))
    print(f"Pet created: {pet}")
except ValidationError as e:
    for error in e.errors:
        print(f"Validation error in {error.field}: {error.message}")
except AuthenticationError as e:
    print(f"Authentication failed: {e}")
except ConflictError as e:
    print(f"Resource conflict: {e}")
```

## Testing Error Scenarios

Create comprehensive error tests:

```bash
# Test validation errors
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{
    "name": "",
    "kind": {"type": "cat", "lives": 0},
    "age": 200
  }'

# Test authentication errors
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer invalid-key" \
  -d '{"name": "Test", "kind": {"type": "dog", "breed": "Test"}}'

# Test conflict errors
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "Duplicate", "kind": {"type": "dog", "breed": "Test"}}'

curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{"name": "Duplicate", "kind": {"type": "cat", "lives": 5}}'

# Test business rule violations
curl -X POST http://localhost:3000/pets.create \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer demo-api-key" \
  -d '{
    "name": "Baby", 
    "kind": {"type": "dog", "breed": "Puppy"},
    "age": 0,
    "behaviors": [{"Aggressive": {"level": 0.8, "notes": "Test"}}]
  }'
```

## What You've Accomplished

✅ **Hierarchical error types** with specific error codes and messages  
✅ **HTTP status code mapping** via the StatusCode trait  
✅ **Structured validation errors** with field-level details  
✅ **Error logging and monitoring** with structured telemetry  
✅ **Error recovery** with retry logic and graceful degradation  
✅ **Client-side error handling** with typed exceptions  
✅ **Comprehensive error testing** scenarios  

## Benefits of `reflectapi`'s Error System

1. **Type Safety**: All errors are compile-time checked
2. **Consistency**: Same error structure across all client languages
3. **Rich Context**: Detailed error information for debugging
4. **HTTP Compliance**: Proper status codes and headers
5. **Developer Experience**: Clear, actionable error messages
6. **Monitoring**: Structured errors for observability tools

## Next Steps

Your API now has production-ready error handling! Let's finish the tutorial by [testing your API](./testing.md) with generated clients and comprehensive test scenarios.
