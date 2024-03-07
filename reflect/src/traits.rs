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
macro_rules! impl_reflect_simple {
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

impl_reflect_simple!(i8, "8-bit signed integer");
impl_reflect_simple!(i16, "16-bit signed integer");
impl_reflect_simple!(i32, "32-bit signed integer");
impl_reflect_simple!(i64, "64-bit signed integer");
impl_reflect_simple!(i128, "128-bit signed integer");
impl_reflect_simple!(u8, "8-bit unsigned integer");
impl_reflect_simple!(u16, "16-bit unsigned integer");
impl_reflect_simple!(u32, "32-bit unsigned integer");
impl_reflect_simple!(u64, "64-bit unsigned integer");
impl_reflect_simple!(u128, "128-bit unsigned integer");
impl_reflect_simple!(f32, "32-bit floating point number");
impl_reflect_simple!(f64, "64-bit floating point number");
impl_reflect_simple!(bool, "Boolean value");
impl_reflect_simple!(char, "Unicode character");
impl_reflect_simple!(std::string::String, "UTF-8 encoded string");

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

fn reflect_type_tuple(schema: &mut crate::Schema, count: usize) -> String {
    let type_name = format!("std::tuple::Tuple{}", count);
    if schema.reserve_type(&type_name) {
        let parameters = (1..(count + 1)).map(|i| format!("T{}", i).into()).collect();
        let type_def = crate::Primitive::new(
            type_name.clone(),
            format!("Tuple holding {} elements", count),
            parameters,
        );
        schema.insert_type(type_def.into());
    }
    type_name
}
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}
macro_rules! impl_reflect_tuple {
    ( $( $name:ident )+)  => {
        impl<$($name: Input),+> Input for ($($name,)+)
        {
            fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
                let type_name = reflect_type_tuple(schema, count!($($name)*));
                crate::TypeReference::new(
                    type_name,
                    vec![$($name::reflect_input_type(schema)),+],
                )
            }
        }

        impl<$($name: Output),+> Output for ($($name,)+)
        {
            fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
                let type_name = reflect_type_tuple(schema, count!($($name)*));
                crate::TypeReference::new(
                    type_name,
                    vec![$($name::reflect_output_type(schema)),+],
                )
            }
        }
    };
}

impl_reflect_tuple! { A }
impl_reflect_tuple! { A B }
impl_reflect_tuple! { A B C }
impl_reflect_tuple! { A B C D }
impl_reflect_tuple! { A B C D E }
impl_reflect_tuple! { A B C D E F }
impl_reflect_tuple! { A B C D E F G }
impl_reflect_tuple! { A B C D E F G H }
impl_reflect_tuple! { A B C D E F G H I }
impl_reflect_tuple! { A B C D E F G H I J }
impl_reflect_tuple! { A B C D E F G H I J K }
impl_reflect_tuple! { A B C D E F G H I J K L }

fn reflect_type_array(schema: &mut crate::Schema) -> String {
    let type_name = "std::array::Array";
    if schema.reserve_type(&type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            format!("Fixed-size Array"),
            vec!["T".into(), "N".to_string().into()],
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<T: Input, const N: usize> Input for [T; N] {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_array(schema),
            vec![T::reflect_input_type(schema), N.to_string().into()],
        )
    }
}
impl<T: Output, const N: usize> Output for [T; N] {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_array(schema),
            vec![T::reflect_output_type(schema), N.to_string().into()],
        )
    }
}

fn reflect_type_pointer(schema: &mut crate::Schema, type_name: &str) -> String {
    if schema.reserve_type(&type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Pointer type".into(),
            vec!["T".into()],
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<T: Input> Input for std::sync::Arc<T> {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_pointer(schema, "std::sync::Arc"),
            vec![T::reflect_input_type(schema)],
        )
    }
}
impl<T: Output> Output for std::sync::Arc<T> {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_pointer(schema, "std::sync::Arc"),
            vec![T::reflect_output_type(schema)],
        )
    }
}
