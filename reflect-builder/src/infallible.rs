use crate::ToStatusCode;

impl ToStatusCode for std::convert::Infallible {
    fn to_status_code(&self) -> u16 {
        500
    }
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
pub struct Infallible;

impl ToStatusCode for Infallible {
    fn to_status_code(&self) -> u16 {
        500
    }
}

impl std::fmt::Display for Infallible {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "internal error: not expected to fail")
    }
}
