#[derive(Debug, Default, Clone)]
#[cfg_attr(
    any(feature = "serde", feature = "builder"),
    derive(serde::Deserialize, serde::Serialize)
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
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            format!("{}::Empty", std::module_path!()).as_str(),
            "Struct object with no fields",
            None,
        )
    }
}

impl crate::Output for Empty {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::reflect_type_simple(
            schema,
            format!("{}::Empty", std::module_path!()).as_str(),
            "Struct object with no fields",
            None,
        )
    }
}