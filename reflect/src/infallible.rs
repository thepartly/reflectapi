use crate::ToStatusCode;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Infallible {}

#[cfg(feature = "builder")]
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

impl Default for Infallible {
    fn default() -> Self {
        Self {}
    }
}

impl From<()> for Infallible {
    fn from(_: ()) -> Self {
        Self::default()
    }
}

impl crate::Input for Infallible {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            format!("{}::Infallible", std::module_path!()).as_str(),
            "Error object which is expected to be never returned",
            None,
        )
    }
}

impl crate::Output for Infallible {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            format!("{}::Infallible", std::module_path!()).as_str(),
            "Error object which is expected to be never returned",
            None,
        )
    }
}
