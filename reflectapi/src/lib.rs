//! # `reflectapi` - is a library and a toolkit for writing web API services in Rust and generating compatible clients,
//! delivering great development experience and efficiency.
//!
//! ## Quick start
//!
//! Server application side example:
//! (complete main.rs example can be found in [https://github.com/thepartly/reflectapi/tree/main/reflectapi-demo](https://github.com/thepartly/reflectapi/tree/main/reflectapi-demo))
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
//! pub fn builder() -> reflectapi::Builder<std::sync::Arc<AppState>> {
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
//!
//!     #[derive(serde::Serialize, reflectapi::Output)]
//!     pub enum BooksListError {
//!         Unauthorized,
//!         LimitExceeded { requested: u32, allowed: u32 },
//!     }
//!
//!     impl reflectapi::StatusCode for BooksListError {
//!         fn status_code(&self) -> http::StatusCode {
//!             match self {
//!                 BooksListError::Unauthorized => http::StatusCode::UNAUTHORIZED,
//!                 BooksListError::LimitExceeded { .. } => http::StatusCode::UNPROCESSABLE_ENTITY,
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! Generated client in Typescript (one of the languages supported by the codegen) example:
//!
//! ```typescript
//! import { client, match } from './generated';
//
//! async function main() {
//!     const c = client('http://localhost:3000');
//
//!     const result = await c.books.list({}, {
//!         authorization: 'password'
//!     })
//!     let { items, pagination } = result.unwrap_ok_or_else((e) => {
//!         throw match(e.unwrap(), {
//!             Unauthorized: () => 'NotAuthorized',
//!             LimitExceeded: ({ requested, allowed }) => `Limit exceeded: ${requested} > ${allowed}`,
//!         });
//!     });
//!     console.log(`items: ${items[0]?.author}`);
//!     console.log(`next cursor: ${pagination.next_cursor}`);
//! }
//
//! main()
//!     .then(() => console.log('done'))
//!     .catch((err) => console.error(err));
//! ```
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
mod option;
mod traits;
mod validation;

#[doc(hidden)]
#[cfg(feature = "rt")]
pub mod rt;

#[cfg(feature = "builder")]
pub(crate) mod builder;

#[cfg(feature = "axum")]
pub mod axum;

#[doc(hidden)]
#[cfg(feature = "codegen")]
pub mod codegen;

#[cfg(feature = "builder")]
pub use builder::{
    BuildError, BuildErrors, Builder, ContentType, Handler, HandlerCallback, HandlerFuture,
    IntoResult, RouteBuilder, Router, StatusCode,
};
pub use empty::*;
pub use infallible::*;
pub use option::*;
pub use reflectapi_derive::{Input, Output};
pub use traits::*;
pub use validation::*;

// Hidden re-exports
// #[doc(hidden)]
// pub use builder::*;
#[doc(hidden)]
pub use reflectapi_schema::*;
