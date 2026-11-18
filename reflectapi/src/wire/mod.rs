//! How serde lays out a reflected type's keys on the wire.
//!
//! The builder reads it for the headers a route's handler is given, and the
//! code generators for the keys they describe, so both follow one set of
//! rules, for `#[serde(flatten)]` above all.

// The builder reads only the headers; the rest is for the code generators.
#![cfg_attr(not(feature = "codegen"), allow(dead_code))]

pub(crate) mod headers;
pub(crate) mod key;
pub(crate) mod layout;
pub(crate) mod shape;
