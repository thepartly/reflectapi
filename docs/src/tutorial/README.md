# Building a Pet Store API

In this tutorial, you'll build a complete Pet Store API from scratch. By the end, you'll have a fully functional web service with automatic client generation, OpenAPI documentation, and proper error handling.

## What You'll Build

A Pet Store API with these features:
- **CRUD operations** for managing pets
- **Authentication** via API keys
- **Validation** for input data
- **Error handling** with proper HTTP status codes
- **Automatic client generation** for TypeScript, Rust, and Python
- **OpenAPI documentation** 

## What You'll Learn

Throughout this tutorial, you'll discover:
- How to define types with ReflectAPI's derive macros
- Creating API endpoints with the canonical handler signature
- Implementing validation and comprehensive error handling
- Working with ReflectAPI's three-state option type
- Generating and using clients in multiple languages
- Testing your API with generated clients

## Prerequisites

- Basic Rust knowledge (structs, enums, error handling)
- Familiarity with async/await concepts
- Understanding of REST API principles

## Tutorial Structure

1. **[Defining Types](./defining-types.md)** - Create the data models for pets, requests, and responses
2. **[Creating Endpoints](./creating-endpoints.md)** - Build API handlers with proper signatures  
3. **[Adding Validation](./adding-validation.md)** - Implement input validation and business rules
4. **[Error Handling](./error-handling.md)** - Handle errors with appropriate HTTP status codes
5. **[Testing Your API](./testing.md)** - Generate clients and test your API

## Getting Started

Create a new Rust project for your Pet Store API:

```bash
cargo new pet-store-api
cd pet-store-api
```

Add the required dependencies to your `Cargo.toml`:

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["builder", "axum", "chrono"] }
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors"] }
anyhow = "1.0"

[dev-dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1.0"
```


Let's start by [defining the types](./defining-types.md) for our Pet Store API!
