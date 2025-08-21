# Validation and Error Handling

Learn how to add robust validation and error handling to your ReflectAPI applications.

## Input Validation

### Using Serde Validation

ReflectAPI works seamlessly with serde's built-in validation:

```rust,ignore
use reflectapi::{Input, Output};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Input)]
struct CreateUserRequest {
    #[serde(deserialize_with = "validate_email")]
    email: String,
    
    #[serde(deserialize_with = "validate_username")]
    username: String,
    
    age: u8, // Automatically validates range 0-255
}

fn validate_email<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let email = String::deserialize(deserializer)?;
    if !email.contains('@') {
        return Err(serde::de::Error::custom("Invalid email format"));
    }
    Ok(email)
}

fn validate_username<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let username = String::deserialize(deserializer)?;
    if username.len() < 3 {
        return Err(serde::de::Error::custom("Username must be at least 3 characters"));
    }
    Ok(username)
}
```

### Custom Validation with NewTypes

Create strongly-typed wrappers for validation:

```rust,ignore
use std::str::FromStr;

#[derive(Serialize, Deserialize)]
pub struct Email(String);

impl FromStr for Email {
    type Err = &'static str;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('@') && s.contains('.') {
            Ok(Email(s.to_string()))
        } else {
            Err("Invalid email format")
        }
    }
}

// Use in your API types
#[derive(Serialize, Deserialize, Input)]
struct CreateUserRequest {
    email: Email,
    username: String,
}
```

## Error Handling

### Defining Error Types

Create comprehensive error types for your API:

```rust,ignore
use reflectapi::{Input, Output};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Output)]
#[serde(tag = "type", content = "details")]
pub enum UserError {
    NotFound { id: u32 },
    ValidationError { field: String, message: String },
    DatabaseError { code: String },
    Unauthorized,
}

#[derive(Serialize, Deserialize, Output)]
pub struct ErrorResponse {
    pub error: UserError,
    pub timestamp: String,
    pub request_id: String,
}
```

### Handler Error Handling

Implement error handling in your route handlers:

```rust,ignore
async fn create_user(
    _state: (),
    req: CreateUserRequest,
    _headers: (),
) -> Result<User, ErrorResponse> {
    // Validate business rules
    if req.age < 18 {
        return Err(ErrorResponse {
            error: UserError::ValidationError {
                field: "age".to_string(),
                message: "Must be 18 or older".to_string(),
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id: uuid::Uuid::new_v4().to_string(),
        });
    }
    
    // Database operation
    match save_user(&req).await {
        Ok(user) => Ok(user),
        Err(e) => Err(ErrorResponse {
            error: UserError::DatabaseError {
                code: e.to_string(),
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
            request_id: uuid::Uuid::new_v4().to_string(),
        }),
    }
}
```

### HTTP Status Codes

Map errors to appropriate HTTP status codes:

```rust,ignore
use axum::{http::StatusCode, response::{IntoResponse, Response}};

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = match &self.error {
            UserError::NotFound { .. } => StatusCode::NOT_FOUND,
            UserError::ValidationError { .. } => StatusCode::BAD_REQUEST,
            UserError::DatabaseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            UserError::Unauthorized => StatusCode::UNAUTHORIZED,
        };
        
        (status, axum::Json(self)).into_response()
    }
}
```

## Client-Side Error Handling

Generated clients provide structured error handling:

### TypeScript

```typescript
import { ApiClient, UserError } from './clients/typescript';

const client = new ApiClient('https://api.example.com');

try {
  const user = await client.users.create({
    email: 'invalid-email',
    username: 'ab',
    age: 16
  });
} catch (error) {
  if (error.status === 400) {
    // Handle validation errors
    const errorData = error.data as { error: UserError };
    
    if (errorData.error.type === 'ValidationError') {
      console.log(`Validation failed: ${errorData.error.details.message}`);
    }
  }
}
```

### Python

```python
from clients.python import AsyncClient, UserError
from httpx import HTTPStatusError

client = AsyncClient(base_url='https://api.example.com')

try:
    user = await client.users.create(CreateUserRequest(
        email='invalid-email',
        username='ab',
        age=16
    ))
except HTTPStatusError as e:
    if e.response.status_code == 400:
        error_data = e.response.json()
        if error_data['error']['type'] == 'ValidationError':
            print(f"Validation failed: {error_data['error']['details']['message']}")
```

### Rust

```rust,ignore
use clients::rust::Client;

let client = Client::new("https://api.example.com");

match client.users().create(CreateUserRequest {
    email: "invalid-email".to_string(),
    username: "ab".to_string(),
    age: 16,
}).await {
    Ok(user) => println!("User created: {:?}", user),
    Err(e) => match e {
        ClientError::Api { status: 400, body } => {
            if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
                match error_response.error {
                    UserError::ValidationError { field, message } => {
                        println!("Validation error on {}: {}", field, message);
                    }
                    _ => println!("Other API error: {:?}", error_response),
                }
            }
        }
        ClientError::Network(e) => {
            println!("Network error: {}", e);
        }
    }
}
```

## Advanced Validation Patterns

### Field-Level Validation

```rust,ignore
use validator::{Validate, ValidationError};

#[derive(Serialize, Deserialize, Input, Validate)]
struct CreateUserRequest {
    #[validate(email)]
    email: String,
    
    #[validate(length(min = 3, max = 20))]
    username: String,
    
    #[validate(range(min = 18, max = 120))]
    age: u8,
    
    #[validate(custom = "validate_password")]
    password: String,
}

fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("Password must be at least 8 characters"));
    }
    if !password.chars().any(|c| c.is_numeric()) {
        return Err(ValidationError::new("Password must contain at least one number"));
    }
    Ok(())
}
```

### Conditional Validation

```rust,ignore
#[derive(Serialize, Deserialize, Input)]
struct UpdateUserRequest {
    email: Option<String>,
    username: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    confirm_password: Option<String>,
}

// Validate in handler
async fn update_user(req: UpdateUserRequest) -> Result<User, ErrorResponse> {
    // Conditional validation
    if let (Some(password), Some(confirm)) = (&req.password, &req.confirm_password) {
        if password != confirm {
            return Err(ErrorResponse {
                error: UserError::ValidationError {
                    field: "confirm_password".to_string(),
                    message: "Passwords do not match".to_string(),
                },
                // ... other fields
            });
        }
    }
    
    // Continue with update...
}
```

## Testing Validation

### Unit Tests

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_email_validation() {
        let valid_req = CreateUserRequest {
            email: "user@example.com".to_string(),
            username: "testuser".to_string(),
            age: 25,
        };
        
        // Should deserialize successfully
        let json = serde_json::to_string(&valid_req).unwrap();
        let parsed: CreateUserRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.email, "user@example.com");
        
        // Invalid email should fail
        let invalid_json = r#"{"email": "invalid", "username": "test", "age": 25}"#;
        assert!(serde_json::from_str::<CreateUserRequest>(invalid_json).is_err());
    }
}
```

### Integration Tests

```rust,ignore
#[tokio::test]
async fn test_validation_errors() {
    let app = create_test_app().await;
    
    let response = app
        .post("/users")
        .json(&serde_json::json!({
            "email": "invalid-email",
            "username": "ab",
            "age": 16
        }))
        .send()
        .await;
    
    assert_eq!(response.status(), 400);
    
    let error: ErrorResponse = response.json().await;
    assert!(matches!(error.error, UserError::ValidationError { .. }));
}
```

## Best Practices

1. **Fail Fast**: Validate input as early as possible in your request pipeline
2. **Clear Messages**: Provide helpful error messages that guide users to fix issues
3. **Consistent Format**: Use a consistent error response format across your API
4. **Documentation**: Document validation rules in your API schema
5. **Testing**: Thoroughly test both valid and invalid input scenarios

## Next Steps

- Learn about [Working with Custom Types](./custom-types.md)
- Explore [Performance Optimization](./performance.md)
- See [Adding Middleware](./middleware.md) for cross-cutting concerns