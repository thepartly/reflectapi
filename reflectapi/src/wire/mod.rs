//! How serde lays out a reflected type: its shape, the keys each field writes,
//! what a `#[serde(flatten)]` field adds to its parent, and so the headers a
//! request-headers type reads.
//!
//! reflectapi's request builder reads it for the headers a route's handler is
//! given, and its code generators for the keys they describe, so the server
//! and its clients follow one set of rules. What the schema promises a client
//! beyond its layout is [`crate::contract`]. Hidden: not a stable API.

pub mod headers;
pub mod key;
pub mod layout;
pub mod shape;
