//! Types for one style of API, opinionated where reflectapi itself is not:
//! cursor pagination, identity envelopes, and an `if` clause that says what
//! to do when a write conflicts with what exists.
//!
//! Their schema names are under `reflectapi::conventions`, so services using
//! them share the same types in generated clients.

pub mod conflict;
pub mod identity;
pub mod pagination;

pub use identity::{Identity, IdentityData};
pub use pagination::{ForwardPaginatedList, ForwardPagination, NonPaginatedList, PageRequest};
