# Using Generic Types

Learn how to work with generic types in `reflectapi` to create flexible, reusable API components while maintaining type safety across all generated clients.

## Overview

`reflectapi` provides excellent support for generic types, allowing you to:
- **Create reusable generic structs and enums** with type parameters
- **Build type-safe APIs** with flexible data structures
- **Generate proper generic code** in TypeScript, Python, and Rust clients
- **Compose complex types** from simple generic building blocks

## Basic Generic Types

### Simple Generic Structs

```rust,ignore
use reflectapi::{Input, Output};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Container<T>
where
    T: Input + Output,
{
    pub value: T,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Response<T>
where
    T: Input + Output,
{
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
}
```

### Multiple Type Parameters

```rust,ignore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct KeyValue<K, V>
where
    K: Input + Output,
    V: Input + Output,
{
    pub key: K,
    pub value: V,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Mapping<K, V>
where
    K: Input + Output,
    V: Input + Output,
{
    pub pairs: Vec<KeyValue<K, V>>,
    pub default_value: Option<V>,
}
```

## Common Generic Patterns

### Paginated Results

```rust,ignore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Page<T>
where
    T: Input + Output,
{
    /// Items in this page
    pub items: Vec<T>,
    /// Current page number (1-based)
    pub page: u32,
    /// Items per page
    pub per_page: u32,
    /// Total number of pages
    pub total_pages: u32,
    /// Total number of items across all pages
    pub total_items: u64,
    /// Cursor for next page (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}
```

### Result Wrappers

```rust,ignore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "status")]
pub enum ApiResult<T, E>
where
    T: Input + Output,
    E: Input + Output,
{
    Success { 
        data: T,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<std::collections::HashMap<String, String>>,
    },
    Error { 
        error: E,
        error_code: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

// Specialized error types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct SystemError {
    pub message: String,
    pub trace_id: String,
}

// Usage
type UserResult = ApiResult<User, ValidationError>;
type SystemResult<T> = ApiResult<T, SystemError>;
```

### Generic Collections

```rust,ignore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Collection<T>
where
    T: Input + Output,
{
    pub items: Vec<T>,
    pub total_count: u64,
    pub filters_applied: Vec<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Tree<T>
where
    T: Input + Output,
{
    pub value: T,
    pub children: Vec<Tree<T>>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Graph<N, E>
where
    N: Input + Output,
    E: Input + Output,
{
    pub nodes: Vec<Node<N>>,
    pub edges: Vec<Edge<E>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Node<T>
where
    T: Input + Output,
{
    pub id: String,
    pub data: T,
    pub position: (f64, f64),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Edge<T>
where
    T: Input + Output,
{
    pub from: String,
    pub to: String,
    pub data: T,
    pub weight: Option<f64>,
}
```

## Generic Enums

### Option-like Types

```rust,ignore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "type")]
pub enum Maybe<T>
where
    T: Input + Output,
{
    Some { value: T },
    None,
    Unknown { reason: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "state", content = "data")]
pub enum AsyncValue<T>
where
    T: Input + Output,
{
    Loading,
    Success(T),
    Error(String),
}
```

### Complex Generic Enums

```rust,ignore
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "operation")]
pub enum DatabaseOperation<T, K>
where
    T: Input + Output,
    K: Input + Output,
{
    Create { data: T },
    Read { key: K },
    Update { key: K, data: T },
    Delete { key: K },
    List { 
        filter: Option<T>,
        limit: Option<u32>,
        offset: Option<u32>,
    },
}
```

## Bounded Generic Types

### Trait Bounds in Practice

```rust,ignore
// Generic type that requires specific traits
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Identifiable<T>
where
    T: Input + Output,
{
    pub id: uuid::Uuid,
    pub data: T,
    pub version: u32,
}

// For types that need comparison
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Comparable<T>
where
    T: Input + Output + PartialEq,
{
    pub current: T,
    pub previous: Option<T>,
    pub changed: bool,
}

// Note: PartialEq is not part of the serde/`reflectapi` requirements,
// but it's needed for the business logic
impl<T> Comparable<T>
where
    T: Input + Output + PartialEq,
{
    pub fn new(current: T, previous: Option<T>) -> Self {
        let changed = match &previous {
            Some(prev) => prev != &current,
            None => true,
        };
        
        Self {
            current,
            previous,
            changed,
        }
    }
}
```

## Generic API Handlers

### Generic CRUD Operations

```rust,ignore
use reflectapi::{Input, Output};
use std::collections::HashMap;

// Generic request types
#[derive(serde::Deserialize, Input)]
pub struct CreateRequest<T>
where
    T: Input + Output,
{
    pub data: T,
}

#[derive(serde::Deserialize, Input)]
pub struct UpdateRequest<T, K>
where
    T: Input + Output,
    K: Input + Output,
{
    pub id: K,
    pub data: T,
}

#[derive(serde::Deserialize, Input)]
pub struct GetRequest<K>
where
    K: Input + Output,
{
    pub id: K,
}

// Generic response types
#[derive(serde::Serialize, Output)]
pub struct EntityResponse<T>
where
    T: Input + Output,
{
    pub entity: T,
    pub metadata: HashMap<String, String>,
}

// Usage with specific types
type CreateUserRequest = CreateRequest<User>;
type CreateUserResponse = EntityResponse<User>;
type UpdateUserRequest = UpdateRequest<User, u32>;
```

## Client-Side Generated Code

### TypeScript

Generated TypeScript maintains full generic type safety:

```typescript
// Generated generic types
export interface Page<T> {
  items: T[];
  page: number;
  per_page: number;
  total_pages: number;
  total_items: number;
  next_cursor?: string;
}

export type ApiResult<T, E> = 
  | { status: "Success"; data: T; metadata?: Record<string, any> }
  | { status: "Error"; error: E; error_code: string; timestamp: string };

export interface Container<T> {
  value: T;
  created_at: string;
}

// Usage with full type safety
const users: Page<User> = await client.users.list({ page: 1, per_page: 20 });
const result: ApiResult<User, ValidationError> = await client.users.create(userData);

if (result.status === "Success") {
  console.log(result.data.name); // TypeScript knows this is User
} else {
  console.log(result.error.field); // TypeScript knows this is ValidationError
}
```

## Next Steps

- Learn about [Working with Custom Types](./custom-types.md) for strong typing strategies  
