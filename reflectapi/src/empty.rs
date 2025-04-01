#[derive(
    Debug, Default, Clone, PartialEq, Eq, Ord, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct Empty {}

impl Empty {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<()> for Empty {
    fn from(_: ()) -> Self {
        Self::new()
    }
}

impl crate::Input for Empty {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflectapi_type_empty(schema, "reflectapi::Empty", "Struct object with no fields")
    }
}

impl crate::Output for Empty {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflectapi_type_empty(schema, "reflectapi::Empty", "Struct object with no fields")
    }
}
