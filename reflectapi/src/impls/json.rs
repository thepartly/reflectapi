use crate::reflectapi_type_hashmap;

impl crate::Input for serde_json::Value {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_type_json_value(schema), vec![])
    }
}

impl crate::Output for serde_json::Value {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(reflectapi_type_json_value(schema), vec![])
    }
}

fn reflectapi_type_json_value(schema: &mut crate::Typespace) -> String {
    let type_name = "serde_json::Value";
    if schema.reserve_type(type_name) {
        let type_def =
            crate::Primitive::new(type_name.into(), "JSON value type".into(), Vec::new(), None);
        schema.insert_type(type_def.into());
    }
    type_name.into()
}

// These impls treat `Map` as `HashMap` rather than `BTreeMap` or `IndexMap` as we cannot assume
// which features the user has enabled. HashMap provides the fewest guarantees about ordering.
impl<K, V> crate::Input for serde_json::Map<K, V>
where
    K: crate::Input,
    V: crate::Input,
{
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_hashmap(schema),
            vec![
                K::reflectapi_input_type(schema),
                V::reflectapi_input_type(schema),
            ],
        )
    }
}

impl<K, V> crate::Output for serde_json::Map<K, V>
where
    K: crate::Output,
    V: crate::Output,
{
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_hashmap(schema),
            vec![
                K::reflectapi_output_type(schema),
                V::reflectapi_output_type(schema),
            ],
        )
    }
}
