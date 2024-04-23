fn reflectapi_uuid(schema: &mut crate::Typespace) -> String {
    let type_name = "uuid::Uuid";
    if schema.reserve_type(&type_name) {
        let type_def =
            crate::Primitive::new(type_name.into(), format!("UUID value type"), vec![], None);
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl crate::Input for uuid::Uuid {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_uuid(schema).into()
    }
}
impl crate::Output for uuid::Uuid {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_uuid(schema).into()
    }
}
