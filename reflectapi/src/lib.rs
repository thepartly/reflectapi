//! # `reflectapi` - is a library and a toolkit for writing web API services in Rust and generating compatible clients,
//! delivering great development experience and efficiency.
//!
//! ## Quick start
//!
//! Define your server
//!
//! ```rust
//!
//! #[derive(Debug)]
//! pub struct AppState {
//!     pub books: Vec<Book>,
//! }
//!
//! async fn books_list(
//!     state: std::sync::Arc<AppState>,
//!     input: proto::Cursor,
//!     headers: proto::Authorization,
//! ) -> Result<proto::Items<Book>, proto::BooksListError> {
//!     unimplemented!("just a demo of API signature")
//! }
//!
//! pub fn builder() -> reflectapi::Builder<Arc<AppState>> {
//!     reflectapi::Builder::new()
//!         .route(books_list, |b| {
//!             b.name("books.list").description("List all books")
//!         })
//! }
//!
//! impl Default for AppState {
//!     fn default() -> Self {
//!         Self {
//!             books: vec![Book {
//!                 isbn: "978-3-16-148410-0".into(),
//!                 title: "The Catcher in the Rye".into(),
//!             }],
//!         }
//!     }
//! }
//!
//! #[derive(
//!    Debug, Clone, serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output,
//! )]
//! pub struct Book {
//!     /// ISBN - identity
//!     pub isbn: String,
//!     /// Title
//!     pub title: String,
//! }
//!
//! pub mod proto {
//!     #[derive(serde::Deserialize, reflectapi::Input)]
//!     pub struct Authorization {
//!         pub authorization: String,
//!     }
//!
//!     #[derive(serde::Deserialize, reflectapi::Input)]
//!     pub struct Cursor {
//!         #[serde(default)]
//!         pub cursor: Option<String>,
//!         #[serde(default)]
//!         pub limit: Option<u32>,
//!     }
//!     #[derive(serde::Serialize, reflectapi::Output)]
//!     pub struct Items<T> {
//!         pub items: Vec<T>,
//!         pub pagination: Pagination,
//!     }
//!
//!     #[derive(serde::Serialize, reflectapi::Output)]
//!     pub struct Pagination {
//!         pub next_cursor: Option<String>,
//!         pub prev_cursor: Option<String>,
//!     }
//! }
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
