//! # `reflectapi`
//!
//! Define your web API in Rust, generate typed TypeScript, Python and Rust clients.
//! Serve it with Axum, Cloudflare Workers or any HTTP server.
//!
//! This is the API reference. The [user guide](https://reflectapi.rs/)
//! is the place to start, and covers what spans many items here:
//! [how it works](https://reflectapi.rs/concepts/how-it-works.html),
//! [what goes over the wire](https://reflectapi.rs/concepts/wire.html),
//! [errors](https://reflectapi.rs/concepts/errors.html),
//! [types](https://reflectapi.rs/concepts/types.html),
//! [the attributes](https://reflectapi.rs/getting-started/attributes.html)
//! and [the generated clients](https://reflectapi.rs/clients/).
//! The whole guide as one file, for coding agents:
//! <https://reflectapi.rs/llms-full.txt>.
//!
//! ## Example
//!
//! ```rust
//! # #[cfg(feature = "builder")]
//! # fn main() {
//! #[derive(serde::Deserialize, reflectapi::Input)]
//! struct GetBook {
//!     isbn: String,
//! }
//!
//! #[derive(serde::Serialize, reflectapi::Output)]
//! struct Book {
//!     isbn: String,
//!     title: String,
//! }
//!
//! async fn get_book(_state: (), input: GetBook, _headers: reflectapi::Empty) -> Option<Book> {
//!     Some(Book { isbn: input.isbn, title: "The Hobbit".into() })
//! }
//!
//! let (schema, routers) = reflectapi::Builder::new()
//!     .name("Books")
//!     .route(get_book, |route| route.name("books.get"))
//!     .build()
//!     .unwrap();
//! // `schema` is what clients are generated from; `routers` are served,
//! // for example with `reflectapi::axum::into_router`.
//! # let _ = (schema, routers);
//! # }
//! # #[cfg(not(feature = "builder"))]
//! # fn main() {}
//! ```
//!
//! ## Where things are
//!
//! - Types: derive [`Input`] for what a route reads and [`Output`] for what it
//!   returns, beside serde's derives. A route's error derives [`ResponseError`].
//! - Routes: [`Builder`] (feature `builder`) registers handlers and builds the
//!   schema and the [`Router`]s.
//! - Serving: [`axum`] (feature `axum`) or [`dispatch`] for any other server.
//! - Types with meaning to clients: [`Option`] (absent, null or a value),
//!   [`Empty`], [`Infallible`].
//! - [`Raw`] (feature `json`): a value kept as the JSON it arrived as.
//! - [`Encoded`] (feature `json`): a typed value encoded into a string, for
//!   clients to pass back without reading, such as a cursor.
//! - Types for one opinionated style of API, such as cursor pagination and
//!   `if` clauses on writes, and lints against the API design standard: the
//!   separate crate [`reflectapi-conventions`](https://docs.rs/reflectapi-conventions).
//! - Generating clients: the `reflectapi` CLI (`cargo binstall reflectapi-cli`),
//!   or `reflectapi::codegen` (feature `codegen`).
//! - The complete example: the
//!   [`reflectapi-demo`](https://github.com/thepartly/reflectapi/tree/main/reflectapi-demo) crate.
//!

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://reflectapi.rs/reflectapi-logo.png",
    html_favicon_url = "https://reflectapi.rs/reflectapi-logo.png"
)]

mod empty;
#[cfg(feature = "json")]
mod encoded;
mod impls;
mod infallible;
mod option;
#[cfg(feature = "json")]
mod raw;
mod traits;
#[cfg(feature = "json")]
mod transcode;
mod validation;

#[cfg(feature = "rt")]
pub mod rt;

#[cfg(feature = "builder")]
pub(crate) mod builder;

#[cfg(feature = "axum")]
pub mod axum;

#[cfg(feature = "builder")]
pub mod dispatch;

#[cfg(feature = "codegen")]
pub mod codegen;

#[cfg(feature = "builder")]
#[allow(deprecated)]
pub use self::builder::StatusCode;
#[cfg(feature = "builder")]
pub use self::builder::{
    BuildError, BuildErrors, BuildMode, Builder, ContentType, DispatchOnly, Event, EventId,
    IntoResult, InvalidEventId, ResponseError, RouteBuilder, Router, StreamItem, WithSchema,
};
#[cfg(feature = "builder")]
use self::builder::{Handler, HandlerCallback};
pub use empty::*;
#[cfg(feature = "json")]
pub use encoded::Encoded;
pub use infallible::*;
pub use option::*;
#[cfg(feature = "json")]
pub use raw::Raw;
#[cfg(feature = "builder")]
pub use reflectapi_derive::ResponseError;
pub use reflectapi_derive::{Input, Output};
pub use traits::*;
pub use validation::*;

// Hidden re-exports
// #[doc(hidden)]
// pub use builder::*;
#[doc(hidden)]
pub use reflectapi_schema::*;

/// What derived code names, not to be used directly.
#[doc(hidden)]
#[cfg(feature = "builder")]
pub mod __private {
    pub use http;
    pub use serde;
}
