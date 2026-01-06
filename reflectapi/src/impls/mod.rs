#[cfg(feature = "uuid")]
mod uuid;

#[cfg(feature = "chrono")]
mod chrono;

#[cfg(feature = "chrono")]
mod chrono_tz;

#[cfg(feature = "url")]
mod url;

#[cfg(feature = "json")]
mod json;

#[cfg(feature = "rust_decimal")]
mod rust_decimal;

#[cfg(feature = "indexmap")]
mod indexmap;
