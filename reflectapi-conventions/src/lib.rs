//! Types for one style of API, opinionated where reflectapi itself is not:
//! cursor pagination, identity envelopes, and an `if` clause that says what
//! to do when a write conflicts with what exists; and [`lints`] that check a
//! schema against the [API design standard](https://reflectapi.rs/design/standard.html).
//!
//! Their Rust paths start with `reflectapi_conventions::`. Rename them to
//! `reflectapi::` on the builder, so every API that uses them gives clients the
//! same names, such as `reflectapi.pagination.PageRequest`;
//! [`lints::conventions_types_renamed`] checks it:
//!
//! ```rust,ignore
//! builder.rename_types("reflectapi_conventions::", "reflectapi::")
//! ```
//!
//! Every generated client has its own copy of the types, a Rust client too, so
//! a client's crate doesn't depend on this one.

pub mod conflict;
pub mod identity;
pub mod lints;
pub mod pagination;

pub use identity::{Identity, IdentityData};
pub use pagination::{ForwardPaginatedList, ForwardPagination, NonPaginatedList, PageRequest};
