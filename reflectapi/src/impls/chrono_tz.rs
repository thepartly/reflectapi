fn reflectapi_timezone(schema: &mut crate::Typespace) -> String {
    let type_name = "chrono_tz::Tz";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "A timezone definition".into(),
            vec![],
            Some("std::string::String".into()),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl crate::Input for chrono_tz::Tz {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_timezone(schema), vec![])
    }
}
impl crate::Output for chrono_tz::Tz {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_timezone(schema), vec![])
    }
}
