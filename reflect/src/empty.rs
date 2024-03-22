#[derive(Debug, Default, Clone, serde::Deserialize, serde::Serialize)]
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
    fn reflect_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_empty(schema, "reflect::Empty", "Struct object with no fields")
    }
}

impl crate::Output for Empty {
    fn reflect_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::reflect_type_empty(schema, "reflect::Empty", "Struct object with no fields")
    }
}
