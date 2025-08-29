//! # ReflectAPI - Code-First Web Service API Declaration and Client Generation
//!
//! ReflectAPI is a Rust library for declaring web service APIs in a code-first manner and generating
//! client libraries for multiple target languages including TypeScript, Rust, and Python.
//!
//! ## Overview
//!
//! ReflectAPI follows a simple principle: define your API using Rust types and traits, then generate
//! everything else automatically. This approach ensures type safety, reduces boilerplate, and keeps
//! your API specification in sync with your implementation.
//!
//! ## Core Concepts
//!
//! ### Input and Output Traits
//!
//! The foundation of ReflectAPI is the [`Input`] and [`Output`] traits. These traits define how
//! types can be used as API inputs and outputs respectively.
//!
//! ```rust
//! use reflectapi::{Input, Output};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Input, Output)]
//! struct User {
//!     id: u32,
//!     name: String,
//!     email: String,
//! }
//! ```
//!
//! ### API Builder
//!
//! Use the [`Builder`] to define your API endpoints and generate schemas:
//!
//! ```rust,no_run
//! # #[cfg(feature = "builder")]
//! # {
//! use reflectapi::{Builder, Input, Output};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Input, Output)]
//! struct User {
//!     id: u32,
//!     name: String,
//! }
//!
//! #[derive(Serialize, Deserialize, Input)]
//! struct CreateUserRequest {
//!     name: String,
//! }
//!
//! // Your handler function (simplified for docs)
//! async fn create_user(_state: (), req: CreateUserRequest, _headers: ()) -> User {
//!     User { id: 1, name: req.name }
//! }
//!
//! // Build the API
//! let builder = Builder::new()
//!     .name("User API")
//!     .description("A simple user management API")
//!     .route(create_user, |route| {
//!         route
//!             .name("users.create")
//!             .description("Create a new user")
//!     });
//!
//! let (schema, routers) = builder.build().unwrap();
//! # }
//! ```
//!
//! ## Code Generation
//!
//! ReflectAPI can generate client libraries for multiple languages:
//!
//! ### TypeScript Client
//! ```bash
//! cargo run --bin reflectapi-cli -- codegen \
//!     --language typescript \
//!     --schema api-schema.json \
//!     --output ./clients/typescript
//! ```
//!
//! ### Rust Client
//! ```bash
//! cargo run --bin reflectapi-cli -- codegen \
//!     --language rust \
//!     --schema api-schema.json \
//!     --output ./clients/rust
//! ```
//!
//! ### Python Client
//! ```bash
//! cargo run --bin reflectapi-cli -- codegen \
//!     --language python \
//!     --schema api-schema.json \
//!     --output ./clients/python
//! ```
//!
//! ## Integration with Web Frameworks
//!
//! ### Axum Integration
//!
//! When the `axum` feature is enabled, you can convert ReflectAPI routers into Axum routers:
//!
//! ```rust,no_run
//! # #[cfg(feature = "axum")]
//! # {
//! use reflectapi::into_router;
//! # use reflectapi::Builder;
//! # let builder = Builder::new();
//! # let (schema, routers) = builder.build().unwrap();
//!
//! let app_state = (); // Your application state
//! let axum_router = into_router(app_state, routers, |_name, router| {
//!     // Add middleware per router if needed
//!     router
//! });
//! # }
//! ```
//!
//! ## OpenAPI Integration
//!
//! ReflectAPI schemas can be converted to OpenAPI specifications:
//!
//! ```rust,no_run
//! # #[cfg(feature = "codegen")]
//! # {
//! # use reflectapi::Builder;
//! # let builder: reflectapi::Builder<()> = Builder::new();
//! # let (schema, _) = builder.build().unwrap();
//! use reflectapi::codegen::openapi::Spec;
//!
//! let openapi_spec = Spec::from(&schema);
//! let openapi_json = serde_json::to_string_pretty(&openapi_spec).unwrap();
//! # }
//! ```
//!
//! ## Examples
//!
//! For complete examples, see the [`reflectapi-demo`](https://github.com/thepartly/reflectapi/tree/main/reflectapi-demo) crate which demonstrates:
//! - Basic CRUD operations
//! - Tagged enums and discriminated unions
//! - Generic types and collections
//! - Error handling
//! - Multiple serialization formats
//!
//! ## Error Handling
//!
//! ReflectAPI provides structured error handling through the [`ValidationError`] type for
//! schema validation and build-time errors.

mod empty;
mod impls;
mod infallible;
mod possible;
mod traits;
mod validation;

#[doc(hidden)]
#[cfg(feature = "rt")]
pub mod rt;

#[cfg(feature = "builder")]
pub use builder::*;

#[cfg(feature = "builder")]
mod builder;

#[cfg(feature = "axum")]
mod axum;

#[doc(hidden)]
#[cfg(feature = "codegen")]
pub mod codegen;

// Public re-exports
#[cfg(feature = "axum")]
pub use axum::*;
pub use empty::*;
pub use infallible::*;
pub use possible::*;
pub use reflectapi_derive::{Input, Output};
pub use traits::*;
pub use validation::*;

// Hidden re-exports
#[doc(hidden)]
pub use reflectapi_schema::*;
