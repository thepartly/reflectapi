//! How serde lays out a reflected type's keys on the wire, and whether it
//! reads `null` into a value.
//!
//! reflectapi's request builder reads it for the headers a route's handler is
//! given, and its code generators for the keys they describe, so both follow
//! one set of rules, for `#[serde(flatten)]` above all. It's public so the
//! reflectapi crates can share it, not as a stable API.

pub mod format;
pub mod headers;
pub mod key;
pub mod layout;
pub mod nullability;
pub mod presence;
pub mod shape;
