pub use serde_derive::{Deserialize, Serialize};

pub trait Serialize {}

pub trait Deserialize<'de>: Sized {}

// Implement for all types so the derive macros can be noops.
impl<T> Serialize for T {}
impl<T> Deserialize<'_> for T {}

pub mod de {
    pub trait DeserializeOwned {}

    impl<T> DeserializeOwned for T where T: for<'de> crate::Deserialize<'de> {}
}
