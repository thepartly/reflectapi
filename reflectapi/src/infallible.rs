#[cfg(feature = "builder")]
use crate::StatusCode;

#[derive(serde::Deserialize, serde::Serialize)]
pub enum Infallible {}

#[cfg(feature = "builder")]
impl StatusCode for Infallible {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl std::fmt::Display for Infallible {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "internal error: not expected to fail")
    }
}

impl crate::Input for Infallible {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflectapi_type_empty(
            schema,
            "reflectapi::Infallible",
            "Error object which is expected to be never returned",
        )
    }
}

impl crate::Output for Infallible {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflectapi_type_empty(
            schema,
            "reflectapi::Infallible",
            "Error object which is expected to be never returned",
        )
    }
}
