fn reflectapi_rust_decimal(schema: &mut crate::Typespace) -> String {
    let type_name = "rust_decimal::Decimal";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Decimal value type".into(),
            vec![],
            Some("std::string::String".into()),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}

impl crate::Input for rust_decimal::Decimal {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_rust_decimal(schema).into()
    }
}

impl crate::Output for rust_decimal::Decimal {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_rust_decimal(schema).into()
    }
}
