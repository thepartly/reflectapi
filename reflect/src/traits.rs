pub trait Input {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference;
}

pub trait Output {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference;
}

fn reflect_type_simple(
    schema: &mut crate::Schema,
    type_name: &str,
    description: &str,
) -> crate::TypeReference {
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(type_name.into(), description.into(), Vec::new());
        schema.insert_type(type_def.into());
    }
    crate::TypeReference::new(type_name.into(), Vec::new())
}
macro_rules! impl_reflect {
    ($type:ty, $description:tt) => {
        impl Input for $type {
            fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
                reflect_type_simple(schema, stringify!($type), $description)
            }
        }
        impl Output for $type {
            fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
                reflect_type_simple(schema, stringify!($type), $description)
            }
        }
    };
}

impl_reflect!(i8, "8-bit signed integer");
impl_reflect!(i16, "16-bit signed integer");
impl_reflect!(i32, "32-bit signed integer");
impl_reflect!(i64, "64-bit signed integer");
impl_reflect!(i128, "128-bit signed integer");
impl_reflect!(u8, "8-bit unsigned integer");
impl_reflect!(u16, "16-bit unsigned integer");
impl_reflect!(u32, "32-bit unsigned integer");
impl_reflect!(u64, "64-bit unsigned integer");
impl_reflect!(u128, "128-bit unsigned integer");
impl_reflect!(f32, "32-bit floating point number");
impl_reflect!(f64, "64-bit floating point number");
impl_reflect!(bool, "Boolean value");
impl_reflect!(char, "Unicode character");
impl_reflect!(std::string::String, "UTF-8 encoded string");

fn reflect_type_vector(schema: &mut crate::Schema) -> String {
    let type_name = "std::vec::Vec";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Expandable array type".into(),
            vec!["T".into()],
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<T: Input> Input for Vec<T> {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_vector(schema),
            vec![T::reflect_input_type(schema)],
        )
    }
}
impl<T: Output> Output for Vec<T> {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_vector(schema),
            vec![T::reflect_output_type(schema)],
        )
    }
}

fn reflect_type_option(schema: &mut crate::Schema) -> String {
    let type_name = "std::option::Option";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Optional / Nullable value type".into(),
            vec!["T".into()],
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<T: Input> Input for Option<T> {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_option(schema),
            vec![T::reflect_input_type(schema)],
        )
    }
}
impl<T: Output> Output for Option<T> {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_option(schema),
            vec![T::reflect_output_type(schema)],
        )
    }
}

fn reflect_type_hashmap(schema: &mut crate::Schema) -> String {
    let type_name = "std::collections::HashMap";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Key-value map type".into(),
            vec!["K".into(), "V".into()],
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<K: Input, V: Input> Input for std::collections::HashMap<K, V> {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_hashmap(schema),
            vec![K::reflect_input_type(schema), V::reflect_input_type(schema)],
        )
    }
}
impl<K: Output, V: Output> Output for std::collections::HashMap<K, V> {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_hashmap(schema),
            vec![
                K::reflect_output_type(schema),
                V::reflect_output_type(schema),
            ],
        )
    }
}
