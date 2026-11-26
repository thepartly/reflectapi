//! What the schema promises a client: whether a field's key may be absent,
//! whether its value may be `null`, what a primitive's string is, in each
//! serialization format, and the length a value must have.
//!
//! The code generators read it to describe each type to a client. It builds
//! on [`crate::wire`], how serde lays out a type, which the server needs too;
//! this module only the tools do, so it's built with them.

pub mod format;
pub mod length;
pub mod nullability;
pub mod presence;
