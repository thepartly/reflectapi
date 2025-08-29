# Working with Custom Types

Learn how to integrate external types and create strongly-typed wrappers for your `reflectapi` schemas.

## Overview

`reflectapi` provides several ways to work with custom types:
- **NewType patterns** for strong typing and validation
- **Feature flags** for common external types (chrono, uuid, url, etc.)
- **Generic wrappers** for reusable type patterns

## NewType Patterns

NewTypes provide compile-time safety and validation without runtime overhead:

### Basic NewTypes

```rust,ignore
use reflectapi::{Input, Output};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(transparent)]
pub struct UserId(pub u32);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(transparent)]
pub struct EmailAddress(pub String);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(transparent)]
pub struct PhoneNumber(pub String);
```

The `#[serde(transparent)]` attribute ensures the newtype serializes as its inner value:

```json
// UserId(123) serializes as:
123

// EmailAddress("user@example.com") serializes as:
"user@example.com"
```

### NewTypes with Validation

Add validation methods to your newtypes:

```rust,ignore
impl UserId {
    pub fn new(id: u32) -> Result<Self, &'static str> {
        if id == 0 {
            Err("User ID cannot be zero")
        } else {
            Ok(UserId(id))
        }
    }
}

impl EmailAddress {
    pub fn new(email: String) -> Result<Self, &'static str> {
        if email.contains('@') {
            Ok(EmailAddress(email))
        } else {
            Err("Invalid email format")
        }
    }
    
    pub fn domain(&self) -> &str {
        self.0.split('@').nth(1).unwrap_or("")
    }
}

impl PhoneNumber {
    pub fn new(phone: String) -> Result<Self, &'static str> {
        let cleaned = phone.chars()
            .filter(|c| c.is_ascii_digit() || *c == '+')
            .collect::<String>();
            
        if cleaned.len() >= 10 {
            Ok(PhoneNumber(cleaned))
        } else {
            Err("Phone number too short")
        }
    }
}
```

### Usage in API Types

```rust,ignore
#[derive(serde::Deserialize, Input)]
pub struct CreateUserRequest {
    pub email: EmailAddress,
    pub phone: Option<PhoneNumber>,
}

#[derive(serde::Serialize, Output)]
pub struct User {
    pub id: UserId,
    pub email: EmailAddress,
    pub phone: Option<PhoneNumber>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

With NewTypes, validation happens automatically during deserialization, providing type safety at the API boundary.


## External Type Support via Feature Flags

`reflectapi` provides built-in support for common external types via feature flags:

### Chrono Types

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["chrono"] }
chrono = { version = "0.4", features = ["serde"] }
```

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Event {
    pub name: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub date: chrono::NaiveDate,
}
```

### UUID Support

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["uuid"] }
uuid = { version = "1.0", features = ["serde"] }
```

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Resource {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

### URL Support

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["url"] }
url = { version = "2.0", features = ["serde"] }
```

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct WebResource {
    pub url: url::Url,
    pub title: String,
    pub description: Option<String>,
}
```

### Decimal Support

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["rust_decimal"] }
rust_decimal = { version = "1.35", features = ["serde"] }
```

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Price {
    pub amount: rust_decimal::Decimal,
    pub currency: String,
}
```


## Best Practices

### 1. Use NewTypes for Domain Modeling

```rust,ignore
// ❌ Primitive obsession
#[derive(Input, Output)]
pub struct User {
    pub id: u32,           // Which ID is this?
    pub email: String,     // Any string?
    pub age: u8,          // What about validation?
}

// ✅ Strong typing with newtypes
#[derive(Input, Output)]
pub struct User {
    pub id: UserId,
    pub email: EmailAddress,
    pub age: Age,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(transparent)]
pub struct Age(u8);

impl Age {
    pub fn new(age: u8) -> Result<Self, &'static str> {
        if age > 150 {
            Err("Age cannot exceed 150")
        } else {
            Ok(Age(age))
        }
    }
}
```

### 2. Implement Display and FromStr

```rust,ignore
impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for UserId {
    type Err = std::num::ParseIntError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserId(s.parse()?))
    }
}
```

### 3. Add Convenience Methods

```rust,ignore
impl EmailAddress {
    pub fn username(&self) -> &str {
        self.0.split('@').next().unwrap_or("")
    }
    
    pub fn domain(&self) -> &str {
        self.0.split('@').nth(1).unwrap_or("")
    }
    
    pub fn is_corporate(&self) -> bool {
        !["gmail.com", "yahoo.com", "outlook.com"]
            .contains(&self.domain())
    }
}
```

### 4. Use Consistent Naming

```rust,ignore
// ✅ Consistent naming pattern
pub struct UserId(pub u32);
pub struct ProductId(pub u32);
pub struct OrderId(pub u32);

// ✅ Consistent validation pattern
impl UserId {
    pub fn new(id: u32) -> Result<Self, ValidationError> { /* */ }
}
```

## Testing Custom Types

Test both serialization and validation:

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_email_validation() {
        assert!(EmailAddress::new("user@example.com".into()).is_ok());
        assert!(EmailAddress::new("invalid-email".into()).is_err());
    }
    
    #[test]
    fn test_email_serialization() {
        let email = EmailAddress("user@example.com".into());
        let json = serde_json::to_string(&email).unwrap();
        assert_eq!(json, "\"user@example.com\"");
        
        let deserialized: EmailAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.0, "user@example.com");
    }
    
    #[test]
    fn test_user_id_from_str() {
        let id: UserId = "123".parse().unwrap();
        assert_eq!(id.0, 123);
    }
}
```

## Client-Side Usage

### TypeScript

Generated TypeScript maintains type safety:

```typescript
// Generated types
export type UserId = number;
export type EmailAddress = string;

export interface User {
  id: UserId;
  email: EmailAddress;
  phone?: PhoneNumber;
}

// Usage with type safety
const user: User = {
  id: 123,
  email: "user@example.com",
  phone: "+1234567890"
};
```

### Python

Generated Python uses type aliases:

```python
from typing import NewType, Optional
from pydantic import BaseModel

UserId = NewType('UserId', int)
EmailAddress = NewType('EmailAddress', str)
PhoneNumber = NewType('PhoneNumber', str)

class User(BaseModel):
    id: UserId
    email: EmailAddress
    phone: Optional[PhoneNumber] = None

# Note: Validation happens server-side in reflectapi.
# Client-side validation can be added manually if needed.
```

## Common Pitfalls

### 1. Forgetting Transparent Serialization

```rust,ignore
// ❌ Will serialize as {"0": 123} instead of 123
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct UserId(pub u32);

// ✅ Serializes as 123
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(transparent)]
pub struct UserId(pub u32);
```

### 2. Missing Useful Traits

```rust,ignore
// ❌ Hard to work with
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct UserId(pub u32);

// ✅ Full set of useful traits
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(transparent)]
pub struct UserId(pub u32);
```

## Next Steps

- Learn about [Using Generic Types](./generics.md) for advanced type patterns
