# Working with Tagged Enums

Learn how to use Serde's tagging strategies with `reflectapi` to create type-safe, discriminated unions that translate beautifully to TypeScript and other languages.

## Overview

Tagged enums are one of `reflectapi`'s most powerful features. Rust's enums with Serde's tagging strategies create discriminated unions that maintain type safety across all generated client languages, particularly TypeScript's discriminated unions.

## Serde Tagging Strategies

Serde provides four tagging strategies for enums, each with different serialization formats:

### 1. External Tagging (Default)

The default serialization wraps the variant in an object:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub enum Behavior {
    Calm,
    Aggressive(f64, String),
    Other {
        description: String,
        notes: String,
    },
}
```

**JSON representation:**
```json
// Calm variant
"Calm"

// Aggressive variant  
{"Aggressive": [0.8, "Very aggressive"]}

// Other variant
{"Other": {"description": "Playful", "notes": "Likes toys"}}
```

**Generated TypeScript:**
```typescript
export type Behavior = 
  | "Calm"
  | { Aggressive: [number, string] }
  | { Other: { description: string; notes: string } };
```

### 2. Internal Tagging

Uses a `type` field to discriminate variants:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    /// A dog
    Dog {
        /// breed of the dog
        breed: String,
    },
    /// A cat
    Cat {
        /// lives left
        lives: u8,
    },
}
```

**JSON representation:**
```json
// Dog variant
{"type": "dog", "breed": "Golden Retriever"}

// Cat variant
{"type": "cat", "lives": 9}
```

**Generated TypeScript:**
```typescript
export type Kind = 
  | { type: "dog"; breed: string }
  | { type: "cat"; lives: number };
```

### 3. Adjacent Tagging

Separates the tag and content into different fields:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "type", content = "data")]
pub enum Message {
    Text(String),
    Image { url: String, caption: String },
    Video { url: String, duration: u32 },
}
```

**JSON representation:**
```json
// Text variant
{"type": "Text", "data": "Hello world"}

// Image variant
{"type": "Image", "data": {"url": "image.jpg", "caption": "A photo"}}

// Video variant
{"type": "Video", "data": {"url": "video.mp4", "duration": 120}}
```

**Generated TypeScript:**
```typescript
export type Message = 
  | { type: "Text"; data: string }
  | { type: "Image"; data: { url: string; caption: string } }
  | { type: "Video"; data: { url: string; duration: number } };
```

### 4. Untagged

No discriminator field - relies on structure to determine variant:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<Value>),
}
```

**JSON representation:**
```json
// String variant
"hello"

// Number variant
42.5

// Bool variant
true

// Array variant
["hello", 42.5, true]
```

## Best Practices for Each Strategy

### Internal Tagging - Recommended for APIs

**When to use:**
- REST APIs with discriminated union responses
- Configuration objects with type variants
- State machines with different states

**Advantages:**
- Clean JSON structure
- Excellent TypeScript support
- Easy to understand and debug
- Works well with OpenAPI specifications

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "status")]
pub enum OrderStatus {
    Pending { order_id: String },
    Processing { 
        order_id: String, 
        estimated_completion: String 
    },
    Completed { 
        order_id: String, 
        tracking_number: String 
    },
    Cancelled { 
        order_id: String, 
        reason: String 
    },
}
```

### External Tagging - Good for Events

**When to use:**
- Event systems
- Command patterns
- Simple discriminated unions

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
pub enum UserEvent {
    Login { user_id: u32, timestamp: String },
    Logout { user_id: u32 },
    Purchase { user_id: u32, item_id: u32, amount: f64 },
}
```

### Adjacent Tagging - For Complex Data

**When to use:**
- Complex nested data structures
- When you need consistent field names
- Integration with external systems

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "message_type", content = "payload")]
pub enum WebSocketMessage {
    UserJoined(UserInfo),
    UserLeft(u32),
    ChatMessage { user_id: u32, text: String },
    SystemAlert(String),
}
```

## Advanced Patterns

### Nested Tagged Enums

You can nest tagged enums for complex hierarchies:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "category")]
pub enum Product {
    Electronics {
        #[serde(flatten)]
        device: ElectronicDevice,
        warranty_years: u32,
    },
    Clothing {
        #[serde(flatten)]
        garment: ClothingItem,
        material: String,
    },
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "type")]
pub enum ElectronicDevice {
    Laptop { cpu: String, ram_gb: u32 },
    Phone { os: String, storage_gb: u32 },
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "type")]
pub enum ClothingItem {
    Shirt { size: String, color: String },
    Pants { waist: u32, length: u32 },
}
```

### Custom Tag Names

Use different tag field names for clarity:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "action_type")]
pub enum UserAction {
    Create { name: String, email: String },
    Update { id: u32, name: Option<String> },
    Delete { id: u32 },
}

#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "event", content = "details")]
pub enum AuditLog {
    UserAction(UserAction),
    SystemEvent(String),
    Error { code: u32, message: String },
}
```

### Rename Variants

Control how variants appear in JSON:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "payment_method", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentMethod {
    CreditCard { 
        #[serde(rename = "card_number")]
        number: String, 
        expiry: String 
    },
    PayPal { email: String },
    BankTransfer { 
        #[serde(rename = "account_number")]
        account: String, 
        routing: String 
    },
}
```

Results in:
```json
{
  "payment_method": "CREDIT_CARD",
  "card_number": "****-****-****-1234",
  "expiry": "12/25"
}
```

## Client Usage Patterns

### TypeScript Usage

`reflectapi` generates TypeScript discriminated unions that work perfectly with type guards:

```typescript
// Generated type
type OrderStatus = 
  | { status: "Pending"; order_id: string }
  | { status: "Processing"; order_id: string; estimated_completion: string }
  | { status: "Completed"; order_id: string; tracking_number: string }
  | { status: "Cancelled"; order_id: string; reason: string };

// Usage with type guards
function handleOrderStatus(status: OrderStatus) {
  switch (status.status) {
    case "Pending":
      console.log(`Order ${status.order_id} is pending`);
      break;
    case "Processing":
      console.log(`Order ${status.order_id} will complete at ${status.estimated_completion}`);
      break;
    case "Completed":
      console.log(`Order ${status.order_id} shipped with tracking ${status.tracking_number}`);
      break;
    case "Cancelled":
      console.log(`Order ${status.order_id} cancelled: ${status.reason}`);
      break;
  }
}

// Type-safe client usage
const status = await client.orders.getStatus(orderId);
handleOrderStatus(status); // Full type safety!
```

### Python Usage

Python clients use Pydantic unions with discriminators:

```python
from typing import Union, Literal
from pydantic import BaseModel, Field

class PendingOrder(BaseModel):
    status: Literal["Pending"] = "Pending"
    order_id: str

class ProcessingOrder(BaseModel):
    status: Literal["Processing"] = "Processing"
    order_id: str
    estimated_completion: str

OrderStatus = Union[PendingOrder, ProcessingOrder, ...]

# Usage
status = await client.orders.get_status(order_id)
if status.status == "Pending":
    print(f"Order {status.order_id} is pending")
elif status.status == "Processing":
    print(f"Order will complete at {status.estimated_completion}")
```

## Testing Tagged Enums

Test all variants to ensure proper serialization:

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_status_serialization() {
        let pending = OrderStatus::Pending {
            order_id: "ORD-123".to_string(),
        };
        
        let json = serde_json::to_string(&pending).unwrap();
        assert_eq!(json, r#"{"status":"Pending","order_id":"ORD-123"}"#);
        
        let deserialized: OrderStatus = serde_json::from_str(&json).unwrap();
        match deserialized {
            OrderStatus::Pending { order_id } => {
                assert_eq!(order_id, "ORD-123");
            }
            _ => panic!("Wrong variant"),
        }
    }
}
```

## Common Pitfalls

### 1. Mixing Tagging Strategies

Don't mix different tagging strategies in related types:

```rust,ignore
// ❌ Inconsistent - will confuse clients
#[serde(tag = "type")]  // Internal tagging
enum Status { ... }

enum Event {  // External tagging (default)
    StatusChange(Status),
}

// ✅ Consistent tagging
#[serde(tag = "type")]
enum Status { ... }

#[serde(tag = "event_type", content = "data")]
enum Event {
    StatusChange(Status),
}
```

### 2. Empty Variants with Internal Tagging

Be careful with unit variants in internally tagged enums:

```rust,ignore
#[derive(serde::Serialize, serde::Deserialize, Input, Output)]
#[serde(tag = "type")]
pub enum State {
    Loading,        // OK - becomes {"type": "Loading"}
    Error(String),  // ❌ Error - can't mix unit and tuple variants
    Ready { data: Vec<String> }, // OK - struct variant
}

// ✅ Better approach
#[serde(tag = "type")]
pub enum State {
    Loading,
    Error { message: String },  // Convert to struct variant
    Ready { data: Vec<String> },
}
```

### 3. Reserved Field Names

Avoid using tag field names in variant data:

```rust,ignore
// ❌ Will conflict
#[serde(tag = "type")]
enum Message {
    User { 
        type: String,  // Conflicts with discriminator field!
        content: String,
    },
}

// ✅ Use different field name
#[serde(tag = "type")]
enum Message {
    User { 
        user_type: String,  // No conflict
        content: String,
    },
}
```

## Next Steps

- Learn about [Using Generic Types](./generics.md) with tagged enums
- Explore [Working with Custom Types](./custom-types.md) for complex data structures
- See [Validation and Error Handling](./validation.md) for robust error types