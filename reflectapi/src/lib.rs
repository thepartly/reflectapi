//! # `reflectapi` - is a library and a toolkit for writing web API services in Rust and generating compatible clients,
//! delivering great development experience and efficiency.
//!
//! ## Quick start
//!
//! Define your server
//!
//! ```rust
//! asdasdasd
//!
//! ```
//! 
//! Now use it from Typescript (one of the languages supported by the codegen):
//! 
//! ```typescript
//! ```
//!
//!
//! ## Examples
//!
//! For complete examples, see the [`reflectapi-demo`](https://github.com/thepartly/reflectapi/tree/main/reflectapi-demo) crate which demonstrates:
//! - Basic CRUD operations
//! - Tagged enums and discriminated unions
//! - Generic types and collections
//! - Error handling
//! - Multiple serialization formats
//! - Project structure setup
//! - Online docs embedding
//! - And many more features
//!

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
pub(crate) mod builder;

#[cfg(feature = "axum")]
mod axum;

#[doc(hidden)]
#[cfg(feature = "codegen")]
pub mod codegen;

// Public re-exports
#[cfg(feature = "axum")]
pub use axum::*;
#[cfg(feature = "builder")]
pub use builder::{
    BuildError, BuildErrors, Builder, ContentType, Handler, HandlerCallback, HandlerFuture,
    IntoResult, RouteBuilder, Router, StatusCode,
};
pub use empty::*;
pub use infallible::*;
pub use possible::*;
pub use reflectapi_derive::{Input, Output};
pub use traits::*;
pub use validation::*;

// Hidden re-exports
// #[doc(hidden)]
// pub use builder::*;
#[doc(hidden)]
pub use reflectapi_schema::*;
