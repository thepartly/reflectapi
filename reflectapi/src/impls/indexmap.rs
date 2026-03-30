use crate::{reflectapi_type_hashmap, reflectapi_type_hashset};
pub use indexmap::{IndexMap, IndexSet};

fn reflectapi_type_indexset(schema: &mut crate::Typespace) -> String {
    let type_name = "indexmap::IndexSet";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Set type ordered by insertion".into(),
            vec!["V".into()],
            Some(crate::TypeReference::new(
                reflectapi_type_hashset(schema),
                vec!["V".into()],
            )),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}

fn reflectapi_type_indexmap(schema: &mut crate::Typespace) -> String {
    let type_name = "indexmap::IndexMap";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Key-value map type ordered by insertion".into(),
            vec!["K".into(), "V".into()],
            Some(crate::TypeReference::new(
                reflectapi_type_hashmap(schema),
                vec!["K".into(), "V".into()],
            )),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}

impl<K, V, S> crate::Input for IndexMap<K, V, S>
where
    K: crate::Input,
    V: crate::Input,
    S: std::hash::BuildHasher,
{
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_indexmap(schema),
            vec![
                K::reflectapi_input_type(schema),
                V::reflectapi_input_type(schema),
            ],
        )
    }
}

impl<K, V, S> crate::Output for IndexMap<K, V, S>
where
    K: crate::Output,
    V: crate::Output,
    S: std::hash::BuildHasher,
{
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_indexmap(schema),
            vec![
                K::reflectapi_output_type(schema),
                V::reflectapi_output_type(schema),
            ],
        )
    }
}

impl<V, S> crate::Input for IndexSet<V, S>
where
    V: crate::Input,
    S: std::hash::BuildHasher,
{
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_indexset(schema),
            vec![V::reflectapi_input_type(schema)],
        )
    }
}

impl<V, S> crate::Output for IndexSet<V, S>
where
    V: crate::Output,
    S: std::hash::BuildHasher,
{
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_indexset(schema),
            vec![V::reflectapi_output_type(schema)],
        )
    }
}
