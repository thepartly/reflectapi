# Rust Client

The Rust client provides zero-cost abstractions, compile-time type safety, and comprehensive error handling for consuming ReflectAPI services.

## Features

- **Zero-cost abstractions** with compile-time type safety
- **reqwest-based HTTP client** with connection pooling and HTTP/2 support
- **Comprehensive error types** using `thiserror` for structured error handling
- **Serde integration** for efficient serialization/deserialization
- **tokio async runtime** for high-performance concurrent operations
- **Builder pattern** for client configuration

## Requirements

- **Rust 1.70+** (MSRV - minimum supported Rust version)
- **tokio** runtime for async operations
- **reqwest** for HTTP client functionality
- **serde** for JSON serialization

## Generation

Generate a Rust client from your ReflectAPI schema:

```bash
reflectapi-cli codegen --language rust --schema api-schema.json --output clients/rust
```

This creates a complete Rust crate with:
- Type-safe structs for all your API types
- Client implementation with all endpoints
- Comprehensive error types
- Cargo.toml with required dependencies

## Installation

Add the generated client to your `Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
# Generated client dependencies are included automatically
my-api-client = { path = "./clients/rust" }

# Or if you've published the client to crates.io
# my-api-client = "1.0.0"
```

## Basic Usage

### Simple Client

```rust
use my_api_client::{Client, models::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.example.com");
    
    // Type-safe API calls with compile-time checking
    let user = client.users().get(123).await?;
    println!("User: {} ({})", user.name, user.email);
    
    // Create new user
    let create_request = CreateUserRequest {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    let new_user = client.users().create(create_request).await?;
    println!("Created user: {}", new_user.id);
    
    Ok(())
}
```

### Client Builder

```rust
use my_api_client::{ClientBuilder, models::*};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new("https://api.example.com")
        .timeout(Duration::from_secs(30))
        .header("Authorization", "Bearer your-token")
        .header("X-API-Version", "1.0")
        .user_agent("MyApp/1.0")
        .build()?;
    
    let user = client.users().get(123).await?;
    println!("User: {}", user.name);
    
    Ok(())
}
```

## Configuration

### Connection Pooling

```rust
use my_api_client::{Client, ClientBuilder};
use reqwest::ClientBuilder as ReqwestBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Custom reqwest client with connection pooling
    let http_client = ReqwestBuilder::new()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(30))
        .http2_prior_knowledge()
        .build()?;
    
    let client = ClientBuilder::new("https://api.example.com")
        .http_client(http_client)
        .build()?;
    
    // All requests use the same connection pool
    let users = client.users().list().await?;
    println!("Found {} users", users.len());
    
    Ok(())
}
```

### TLS Configuration

```rust
use my_api_client::ClientBuilder;
use reqwest::ClientBuilder as ReqwestBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Custom TLS configuration
    let http_client = ReqwestBuilder::new()
        .danger_accept_invalid_certs(false)  // Production: false
        .danger_accept_invalid_hostnames(false)
        .use_rustls_tls()  // Use rustls instead of native TLS
        .build()?;
    
    let client = ClientBuilder::new("https://api.example.com")
        .http_client(http_client)
        .build()?;
    
    Ok(())
}
```

## Error Handling

The Rust client provides comprehensive error handling using `thiserror`:

```rust
use my_api_client::{Client, ClientError, ApiError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.example.com");
    
    match client.users().get(999).await {
        Ok(user) => println!("Found user: {}", user.name),
        Err(ClientError::Api(api_error)) => {
            match api_error.status() {
                404 => println!("User not found"),
                401 => println!("Authentication required"),
                403 => println!("Access denied"),
                status if status >= 500 => {
                    println!("Server error {}: {}", status, api_error.message());
                }
                _ => println!("API error {}: {}", api_error.status(), api_error.message()),
            }
        }
        Err(ClientError::Network(net_error)) => {
            println!("Network error: {}", net_error);
        }
        Err(ClientError::Serialization(ser_error)) => {
            println!("Serialization error: {}", ser_error);
        }
        Err(e) => println!("Unexpected error: {}", e),
    }
    
    Ok(())
}
```

### Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}

#[derive(Debug, Error)]
#[error("API returned {status}: {message}")]
pub struct ApiError {
    status: u16,
    message: String,
    body: Option<serde_json::Value>,
}

impl ApiError {
    pub fn status(&self) -> u16 {
        self.status
    }
    
    pub fn message(&self) -> &str {
        &self.message
    }
    
    pub fn body(&self) -> Option<&serde_json::Value> {
        self.body.as_ref()
    }
}
```

## Type Safety

The Rust client provides compile-time type safety:

```rust
use my_api_client::{Client, models::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.example.com");
    
    // Compile-time type checking
    let user: User = client.users().get(123).await?;
    
    let create_request = CreateUserRequest {
        name: "Alice".to_string(),        // ✅ Required field
        email: "alice@example.com".to_string(),  // ✅ Required field
        age: Some(25),                    // ✅ Optional field
    };
    
    let new_user: User = client.users().create(create_request).await?;
    
    // Compile errors for invalid types:
    // let invalid_request = CreateUserRequest {
    //     name: 123,  // ❌ Compile error: Expected String
    //     email: "alice@example.com".to_string()
    // };
    
    Ok(())
}
```

### Option Types

```rust
use my_api_client::models::*;

// ReflectAPI Option<T> maps to Rust Option<T>
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,     // Three-state: None, Some(value), or omitted
    pub email: Option<String>,
    pub age: Option<Option<u8>>,  // Optional field that can be null
}

// Usage
let update_request = UpdateUserRequest {
    name: Some("New Name".to_string()),  // Update name
    email: None,                         // Set email to null
    // age field omitted - won't be sent in request
};
```

## Advanced Usage

### Request Middleware

```rust
use my_api_client::{Client, ClientBuilder};
use reqwest::{Request, Response};
use std::time::Instant;

#[derive(Clone)]
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    async fn log_request(&self, req: &Request) {
        println!("→ {} {}", req.method(), req.url());
    }
    
    async fn log_response(&self, res: &Response, duration: std::time::Duration) {
        println!("← {} {} ({:.3}s)", 
                res.status().as_u16(), 
                res.url(), 
                duration.as_secs_f64());
    }
}

// Custom client with middleware
pub struct LoggingClient {
    inner: Client,
    middleware: LoggingMiddleware,
}

impl LoggingClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            inner: Client::new(base_url),
            middleware: LoggingMiddleware,
        }
    }
    
    pub async fn get_user(&self, id: u32) -> Result<User, ClientError> {
        let start = Instant::now();
        // Log and execute request
        let result = self.inner.users().get(id).await;
        let duration = start.elapsed();
        
        println!("Request completed in {:.3}s", duration.as_secs_f64());
        result
    }
}
```

### Concurrent Operations

```rust
use my_api_client::{Client, models::*};
use futures::future::try_join_all;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.example.com");
    
    // Concurrent requests
    let user_ids = vec![1, 2, 3, 4, 5];
    
    let user_futures = user_ids.into_iter()
        .map(|id| client.users().get(id))
        .collect::<Vec<_>>();
    
    let users = try_join_all(user_futures).await?;
    
    for user in users {
        println!("User: {} ({})", user.name, user.email);
    }
    
    Ok(())
}

// Batch operations with error handling
async fn get_users_batch(
    client: &Client,
    user_ids: Vec<u32>
) -> Result<HashMap<u32, User>, ClientError> {
    use futures::stream::{self, StreamExt};
    
    let results = stream::iter(user_ids)
        .map(|id| async move {
            match client.users().get(id).await {
                Ok(user) => Some((id, user)),
                Err(ClientError::Api(api_error)) if api_error.status() == 404 => None,
                Err(e) => return Err(e),
            }
        })
        .buffer_unordered(10)  // Max 10 concurrent requests
        .collect::<Vec<_>>()
        .await;
    
    let mut users = HashMap::new();
    for result in results {
        if let Some((id, user)) = result? {
            users.insert(id, user);
        }
    }
    
    Ok(users)
}
```

### Authentication Handling

```rust
use my_api_client::{Client, ClientBuilder, ClientError};
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthenticatedClient {
    inner: Client,
    token: Arc<RwLock<String>>,
    refresh_token: Arc<RwLock<Option<String>>>,
}

impl AuthenticatedClient {
    pub fn new(base_url: &str, token: String) -> Self {
        let client = ClientBuilder::new(base_url)
            .header("Authorization", format!("Bearer {}", token))
            .build()
            .expect("Failed to build client");
        
        Self {
            inner: client,
            token: Arc::new(RwLock::new(token)),
            refresh_token: Arc::new(RwLock::new(None)),
        }
    }
    
    pub async fn get_user(&self, id: u32) -> Result<User, ClientError> {
        match self.inner.users().get(id).await {
            Ok(user) => Ok(user),
            Err(ClientError::Api(api_error)) if api_error.status() == 401 => {
                // Attempt token refresh
                if self.refresh_token().await? {
                    // Retry with new token
                    self.inner.users().get(id).await
                } else {
                    Err(ClientError::Api(api_error))
                }
            }
            Err(e) => Err(e),
        }
    }
    
    async fn refresh_token(&self) -> Result<bool, ClientError> {
        let refresh_token = self.refresh_token.read().await.clone();
        
        if let Some(refresh_token) = refresh_token {
            // Call refresh endpoint
            let refresh_client = Client::new(&self.inner.base_url());
            
            match refresh_client.auth().refresh(&refresh_token).await {
                Ok(auth_response) => {
                    // Update token
                    let mut token = self.token.write().await;
                    *token = auth_response.access_token;
                    
                    // Update refresh token if provided
                    if let Some(new_refresh) = auth_response.refresh_token {
                        let mut refresh = self.refresh_token.write().await;
                        *refresh = Some(new_refresh);
                    }
                    
                    Ok(true)
                }
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}
```

## Performance Optimization

### Connection Reuse

```rust
use my_api_client::{Client, ClientBuilder};
use reqwest::ClientBuilder as ReqwestBuilder;
use std::sync::Arc;

// Shared client for connection reuse
#[derive(Clone)]
pub struct SharedClient {
    inner: Arc<Client>,
}

impl SharedClient {
    pub fn new(base_url: &str) -> Result<Self, ClientError> {
        let http_client = ReqwestBuilder::new()
            .pool_max_idle_per_host(50)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .build()?;
        
        let client = ClientBuilder::new(base_url)
            .http_client(http_client)
            .build()?;
        
        Ok(Self {
            inner: Arc::new(client),
        })
    }
    
    pub async fn get_user(&self, id: u32) -> Result<User, ClientError> {
        self.inner.users().get(id).await
    }
}

// Usage across multiple tasks
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SharedClient::new("https://api.example.com")?;
    
    let mut handles = vec![];
    
    // Spawn multiple tasks using the same client
    for i in 1..=10 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            client.get_user(i).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        match handle.await? {
            Ok(user) => println!("User: {}", user.name),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

### Streaming Responses

```rust
use my_api_client::{Client, models::*};
use futures::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.example.com");
    
    // Stream large datasets
    let mut user_stream = client.users().list_stream().await?;
    
    while let Some(user_batch) = user_stream.next().await {
        let users = user_batch?;
        
        for user in users {
            println!("Processing user: {}", user.name);
            // Process user without loading all into memory
        }
    }
    
    Ok(())
}
```

## CLI Integration

The Rust client works excellently for building CLI tools:

```rust
use my_api_client::{Client, models::*};
use clap::{Arg, Command};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("user-cli")
        .arg(Arg::new("base-url")
             .long("base-url")
             .value_name("URL")
             .help("API base URL")
             .required(true))
        .arg(Arg::new("token")
             .long("token")
             .value_name("TOKEN")
             .help("Authentication token")
             .required(true))
        .subcommand(Command::new("get")
                   .arg(Arg::new("id")
                        .help("User ID")
                        .required(true)))
        .subcommand(Command::new("list"))
        .get_matches();
    
    let base_url = matches.get_one::<String>("base-url").unwrap();
    let token = matches.get_one::<String>("token").unwrap();
    
    let client = ClientBuilder::new(base_url)
        .header("Authorization", format!("Bearer {}", token))
        .build()?;
    
    match matches.subcommand() {
        Some(("get", sub_matches)) => {
            let id: u32 = sub_matches.get_one::<String>("id")
                .unwrap()
                .parse()?;
            
            match client.users().get(id).await {
                Ok(user) => {
                    println!("User: {} ({})", user.name, user.email);
                    println!("Created: {}", user.created_at);
                }
                Err(ClientError::Api(api_error)) if api_error.status() == 404 => {
                    eprintln!("User not found");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(("list", _)) => {
            let users = client.users().list().await?;
            
            for user in users {
                println!("{:5} {:20} {}", user.id, user.name, user.email);
            }
        }
        _ => {
            eprintln!("Invalid subcommand");
            std::process::exit(1);
        }
    }
    
    Ok(())
}
```

## Testing

### Unit Testing

```rust
use my_api_client::{Client, models::*};
use mockito::{mock, server_url};

#[tokio::test]
async fn test_get_user() {
    // Mock server response
    let _m = mock("GET", "/users/123")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 123, "name": "Alice", "email": "alice@example.com"}"#)
        .create();
    
    let client = Client::new(&server_url());
    let user = client.users().get(123).await.unwrap();
    
    assert_eq!(user.id, 123);
    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
}

#[tokio::test]
async fn test_user_not_found() {
    let _m = mock("GET", "/users/999")
        .with_status(404)
        .with_body(r#"{"error": "User not found"}"#)
        .create();
    
    let client = Client::new(&server_url());
    let result = client.users().get(999).await;
    
    assert!(result.is_err());
    if let Err(ClientError::Api(api_error)) = result {
        assert_eq!(api_error.status(), 404);
    } else {
        panic!("Expected API error");
    }
}
```

### Integration Testing

```rust
use my_api_client::{Client, models::*};

#[tokio::test]
async fn test_user_crud() {
    let client = Client::new("http://localhost:3000");
    
    // Create user
    let create_request = CreateUserRequest {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };
    
    let created_user = client.users().create(create_request).await.unwrap();
    assert_eq!(created_user.name, "Test User");
    assert_eq!(created_user.email, "test@example.com");
    
    // Get user
    let retrieved_user = client.users().get(created_user.id).await.unwrap();
    assert_eq!(retrieved_user.id, created_user.id);
    assert_eq!(retrieved_user.name, created_user.name);
    
    // Update user
    let update_request = UpdateUserRequest {
        name: Some("Updated Name".to_string()),
        email: None,  // Keep existing email
    };
    
    let updated_user = client.users()
        .update(created_user.id, update_request)
        .await
        .unwrap();
    assert_eq!(updated_user.name, "Updated Name");
    
    // Delete user
    client.users().delete(created_user.id).await.unwrap();
    
    // Verify deletion
    let result = client.users().get(created_user.id).await;
    assert!(matches!(result, Err(ClientError::Api(api_error)) if api_error.status() == 404));
}
```

## Troubleshooting

### Common Issues

**Compilation errors:**
```bash
# Ensure you have the latest Rust toolchain
rustup update

# Check for missing dependencies
cargo check

# Update dependencies
cargo update
```

**Runtime errors:**
```rust
// Enable detailed error logging
env_logger::init();

// Or use tracing for structured logging
use tracing::{info, error, debug};

match client.users().get(123).await {
    Ok(user) => info!("Retrieved user: {:?}", user),
    Err(e) => error!("Failed to get user: {:?}", e),
}
```

**TLS/Certificate issues:**
```rust
// Disable certificate verification (development only)
let http_client = reqwest::ClientBuilder::new()
    .danger_accept_invalid_certs(true)
    .build()?;

// Or provide custom CA certificates
let http_client = reqwest::ClientBuilder::new()
    .add_root_certificate(cert)
    .build()?;
```

### Debug Mode

Enable comprehensive debugging:

```rust
use my_api_client::{Client, ClientBuilder};

// Enable request/response logging
let client = ClientBuilder::new("https://api.example.com")
    .debug(true)  // Logs all HTTP requests/responses
    .build()?;

// Or use a custom logger
use reqwest::ClientBuilder as ReqwestBuilder;
use reqwest_middleware::{ClientBuilder as MiddlewareBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;

let http_client = MiddlewareBuilder::new(
    ReqwestBuilder::new().build()?
)
.with(TracingMiddleware::default())
.build();

let client = ClientBuilder::new("https://api.example.com")
    .http_client(http_client)
    .build()?;
```

## Best Practices

1. **Use the builder pattern** for client configuration
2. **Handle all error cases** explicitly 
3. **Reuse clients** for connection pooling
4. **Use appropriate timeouts** for your use case
5. **Leverage Rust's type system** for compile-time correctness
6. **Use `Arc` for sharing clients** across tasks
7. **Implement proper logging** for debugging
8. **Write comprehensive tests** with both unit and integration tests

## Next Steps

- [TypeScript Client Guide](./typescript.md) - Compare with TypeScript implementation
- [Python Client Guide](./python.md) - Compare with Python implementation
- [Client Comparison](./README.md#client-comparison) - Feature comparison across languages