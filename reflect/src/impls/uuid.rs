fn reflect_uuid(schema: &mut crate::Schema) -> String {
    let type_name = "uuid::Uuid";
    if schema.reserve_type(&type_name) {
        let type_def =
            crate::Primitive::new(type_name.into(), format!("UUID value type"), vec![], None);
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl crate::Input for uuid::Uuid {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        reflect_uuid(schema).into()
    }
}
impl crate::Output for uuid::Uuid {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        reflect_uuid(schema).into()
    }
}
