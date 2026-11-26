fn reflectapi_url(schema: &mut crate::Typespace) -> String {
    let type_name = "url::Url";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "URL value type".to_string(),
            vec![],
            Some("std::string::String".into()),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl crate::Input for url::Url {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_url(schema).into()
    }
}
impl crate::Output for url::Url {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_url(schema).into()
    }
}
