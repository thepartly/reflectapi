use crate::StatusCode;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Infallible {
    marker: std::marker::PhantomData<()>,
}

#[cfg(feature = "builder")]
impl StatusCode for Infallible {
    fn status_code(&self) -> u16 {
        500
    }
}

impl std::fmt::Display for Infallible {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "internal error: not expected to fail")
    }
}

impl crate::Input for Infallible {
    fn reflect_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_empty(
            schema,
            "reflect::Infallible",
            "Error object which is expected to be never returned",
        )
    }
}

impl crate::Output for Infallible {
    fn reflect_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_empty(
            schema,
            "reflect::Infallible",
            "Error object which is expected to be never returned",
        )
    }
}
