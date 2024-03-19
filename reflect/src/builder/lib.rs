mod builder;
mod handler;
mod infallible;

pub use builder::*;
pub use handler::*;

pub trait ToStatusCode {
    fn to_status_code(&self) -> u16;
}
