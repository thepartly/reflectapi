# Using Generic Types

Learn how to work with generic types in ReflectAPI to create flexible, reusable API components while maintaining type safety across all generated clients.

## Overview

ReflectAPI provides excellent support for generic types, allowing you to:
- **Create reusable generic structs and enums** with type parameters
- **Build type-safe APIs** with flexible data structures
- **Generate proper generic code** in TypeScript, Python, and Rust clients
- **Compose complex types** from simple generic building blocks

## Basic Generic Types

### Simple Generic Structs

```rust,ignore
use reflectapi::{Input, Output};

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Container<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub value: T,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Response<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
}
```

### Multiple Type Parameters

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct KeyValue<K, V>
where
    K: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
    V: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub key: K,
    pub value: V,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Mapping<K, V>
where
    K: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
    V: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub pairs: Vec<KeyValue<K, V>>,
    pub default_value: Option<V>,
}
```

## Common Generic Patterns

### Paginated Results

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Page<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
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

// Usage in handlers
async fn list_users(
    state: Arc<AppState>,
    request: ListUsersRequest,
    _headers: reflectapi::Empty,
) -> Result<Page<User>, ListUsersError> {
    let users = state.get_users(request.page, request.per_page).await?;
    
    Ok(Page {
        items: users.items,
        page: request.page,
        per_page: request.per_page,
        total_pages: users.total_pages,
        total_items: users.total_count,
        next_cursor: users.next_cursor,
    })
}
```

### Result Wrappers

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "status")]
pub enum ApiResult<T, E>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
    E: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    Success { 
        data: T,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    },
    Error { 
        error: E,
        error_code: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

// Specialized error types
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
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
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Collection<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub items: Vec<T>,
    pub total_count: u64,
    pub filters_applied: Vec<String>,
    pub sort_order: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Tree<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub value: T,
    pub children: Vec<Tree<T>>,
    pub parent_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Graph<N, E>
where
    N: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
    E: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub nodes: Vec<Node<N>>,
    pub edges: Vec<Edge<E>>,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Node<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub id: String,
    pub data: T,
    pub position: (f64, f64),
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Edge<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
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
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "type")]
pub enum Maybe<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    Some { value: T },
    None,
    Unknown { reason: String },
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "state", content = "data")]
pub enum AsyncValue<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    Loading,
    Success(T),
    Error(String),
}
```

### Complex Generic Enums

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "operation")]
pub enum DatabaseOperation<T, K>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
    K: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
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
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Identifiable<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub id: uuid::Uuid,
    pub data: T,
    pub version: u32,
}

// For types that need comparison
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Comparable<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output + PartialEq,
{
    pub current: T,
    pub previous: Option<T>,
    pub changed: bool,
}

// Note: PartialEq is not part of the serde/ReflectAPI requirements,
// but it's needed for the business logic
impl<T> Comparable<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output + PartialEq,
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
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub data: T,
}

#[derive(serde::Deserialize, Input)]
pub struct UpdateRequest<T, K>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
    K: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub id: K,
    pub data: T,
}

#[derive(serde::Deserialize, Input)]
pub struct GetRequest<K>
where
    K: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub id: K,
}

// Generic response types
#[derive(serde::Serialize, Output)]
pub struct EntityResponse<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub entity: T,
    pub metadata: HashMap<String, String>,
}

// Example handlers - you'd register these for specific types
async fn create_entity<T>(
    state: Arc<AppState>,
    request: CreateRequest<T>,
    _headers: reflectapi::Empty,
) -> Result<EntityResponse<T>, CreateEntityError>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output + Clone,
{
    // Implementation would be type-specific
    todo!("Implement based on entity type")
}

// In practice, you'd have specific handlers:
async fn create_user(
    state: Arc<AppState>,
    request: CreateRequest<User>,
    headers: reflectapi::Empty,
) -> Result<EntityResponse<User>, CreateEntityError> {
    // Specific implementation for User
    let user = state.user_service.create(request.data).await?;
    Ok(EntityResponse {
        entity: user,
        metadata: HashMap::new(),
    })
}
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

// Generic collections work perfectly
const userTree: Tree<User> = await client.users.getHierarchy();
const productGraph: Graph<Product, Relationship> = await client.products.getRelationships();
```

### Python

Generated Python uses proper generic typing:

```python
from typing import TypeVar, Generic, List, Optional, Dict, Union
from pydantic import BaseModel

T = TypeVar('T')
E = TypeVar('E')
K = TypeVar('K')
V = TypeVar('V')

class Page(BaseModel, Generic[T]):
    items: List[T]
    page: int
    per_page: int
    total_pages: int
    total_items: int
    next_cursor: Optional[str] = None

class ApiResult(BaseModel, Generic[T, E]):
    # Uses discriminated union
    status: str
    # Additional fields based on status

class Container(BaseModel, Generic[T]):
    value: T
    created_at: str

# Usage with type checking
from myapi import User, ValidationError

users: Page[User] = await client.users.list(page=1, per_page=20)
result: ApiResult[User, ValidationError] = await client.users.create(user_data)

if result.status == "Success":
    print(result.data.name)  # mypy knows this is User
else:
    print(result.error.field)  # mypy knows this is ValidationError
```

### Rust

Generated Rust maintains zero-cost generics:

```rust,ignore
// Generated generic types match the original definitions
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub total_items: u64,
    pub next_cursor: Option<String>,
}

pub enum ApiResult<T, E> {
    Success { 
        data: T,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    },
    Error { 
        error: E,
        error_code: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

// Usage
let users: Page<User> = client.users().list(ListUsersRequest {
    page: 1,
    per_page: 20,
}).await?;

let result: ApiResult<User, ValidationError> = client.users().create(CreateUserRequest {
    data: user_data,
}).await?;

match result {
    ApiResult::Success { data, .. } => {
        println!("Created user: {}", data.name);
    },
    ApiResult::Error { error, .. } => {
        println!("Validation failed on field: {}", error.field);
    },
}
```

## Advanced Generic Patterns

### Generic Builders

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct QueryBuilder<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub filters: Vec<Filter<T>>,
    pub sorts: Vec<Sort>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Filter<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub field: String,
    pub operator: FilterOperator,
    pub value: FilterValue<T>,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "type")]
pub enum FilterValue<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    Single { value: T },
    Multiple { values: Vec<T> },
    Range { min: T, max: T },
    Null,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    StartsWith,
    EndsWith,
    In,
    NotIn,
    Between,
    IsNull,
    IsNotNull,
}
```

### Generic State Machines

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct StateMachine<S, E>
where
    S: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
    E: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub current_state: S,
    pub history: Vec<StateTransition<S, E>>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct StateTransition<S, E>
where
    S: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
    E: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub from_state: S,
    pub to_state: S,
    pub event: E,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// Example: Order state machine
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub enum OrderState {
    Created,
    PaymentPending,
    PaymentReceived,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
    Refunded,
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub enum OrderEvent {
    PaymentRequested,
    PaymentReceived,
    ProcessingStarted,
    Shipped { tracking_number: String },
    Delivered,
    Cancelled { reason: String },
    RefundIssued { amount: rust_decimal::Decimal },
}

type OrderStateMachine = StateMachine<OrderState, OrderEvent>;
```

## Best Practices

### 1. Keep Generic Constraints Minimal

```rust,ignore
// ❌ Too many constraints
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Container<T>
where
    T: serde::Serialize 
        + for<'de> serde::Deserialize<'de> 
        + Input 
        + Output 
        + Clone 
        + Debug 
        + PartialEq 
        + Send 
        + Sync,
{
    pub value: T,
}

// ✅ Minimal constraints for ReflectAPI
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Container<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub value: T,
}

// Add trait bounds only in impl blocks when needed
impl<T> Container<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output + Clone,
{
    pub fn duplicate(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}
```

### 2. Use Type Aliases for Complex Generics

```rust,ignore
// ✅ Clear type aliases
pub type UserPage = Page<User>;
pub type UserResult = ApiResult<User, ValidationError>;
pub type ProductCollection = Collection<Product>;

// Make APIs easier to understand
async fn list_users(
    state: Arc<AppState>,
    request: ListUsersRequest,
    _headers: reflectapi::Empty,
) -> Result<UserPage, ListUsersError> {
    // Implementation
}
```

### 3. Document Generic Parameters

```rust,ignore
/// A paginated collection of items
/// 
/// # Type Parameters
/// 
/// * `T` - The type of items in the collection. Must be serializable and
///         implement ReflectAPI Input/Output traits.
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Page<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    /// Items in this page
    pub items: Vec<T>,
    /// Current page number (1-based)
    pub page: u32,
    /// Items per page
    pub per_page: u32,
}
```

### 4. Provide Concrete Examples

```rust,ignore
// Provide concrete type examples for documentation
pub type ProductPage = Page<Product>;
pub type UserPage = Page<User>;
pub type OrderPage = Page<Order>;

// This helps with:
// 1. Documentation generation
// 2. Client code examples
// 3. Type inference in client languages
```

## Testing Generic Types

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_page_serialization() {
        let page = Page {
            items: vec!["item1".to_string(), "item2".to_string()],
            page: 1,
            per_page: 10,
            total_pages: 1,
            total_items: 2,
            next_cursor: None,
        };
        
        let json = serde_json::to_string(&page).unwrap();
        let deserialized: Page<String> = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.items.len(), 2);
        assert_eq!(deserialized.page, 1);
    }
    
    #[test]
    fn test_api_result_success() {
        let result: ApiResult<String, String> = ApiResult::Success {
            data: "test".to_string(),
            metadata: None,
        };
        
        match result {
            ApiResult::Success { data, .. } => assert_eq!(data, "test"),
            _ => panic!("Expected success"),
        }
    }
}
```

## Common Pitfalls

### 1. Forgetting Lifetime Parameters

```rust,ignore
// ❌ Missing lifetime in where clause
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Container<T>
where
    T: serde::Serialize + serde::Deserialize + Input + Output, // Wrong!
{
    pub value: T,
}

// ✅ Correct lifetime specification
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Container<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub value: T,
}
```

### 2. Over-constraining Generic Types

```rust,ignore
// ❌ Adding unnecessary constraints at struct level
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Container<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output + Clone + Debug,
{
    pub value: T,
}

// ✅ Add constraints only where needed
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub struct Container<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output,
{
    pub value: T,
}

impl<T> Clone for Container<T>
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + Input + Output + Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
        }
    }
}
```

## Next Steps

- Learn about [Working with Tagged Enums](./tagged-enums.md) for discriminated union patterns
- Explore [Working with Custom Types](./custom-types.md) for strong typing strategies  
- See [Validation and Error Handling](./validation.md) for generic error patterns