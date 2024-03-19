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
    fallback: Option<crate::TypeReference>,
) -> crate::TypeReference {
    if schema.reserve_type(type_name) {
        let mut type_def =
            crate::Primitive::new(type_name.into(), description.into(), Vec::new(), None);
        type_def.fallback = fallback;
        schema.insert_type(type_def.into());
    }
    crate::TypeReference::new(type_name.into(), Vec::new())
}
macro_rules! impl_reflect_simple {
    ($type:ty, $description:tt) => {
        impl Input for $type {
            fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
                reflect_type_simple(schema, stringify!($type), $description, None)
            }
        }
        impl Output for $type {
            fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
                reflect_type_simple(schema, stringify!($type), $description, None)
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

impl Input for isize {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        let fallback = Some(i64::reflect_input_type(schema));
        reflect_type_simple(
            schema,
            "isize",
            "Machine-specific-bit signed integer",
            fallback,
        )
    }
}
impl Output for isize {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        let fallback = Some(i64::reflect_output_type(schema));
        reflect_type_simple(
            schema,
            "isize",
            "Machine-specific-bit signed integer",
            fallback,
        )
    }
}

fn reflect_type_vector(schema: &mut crate::Schema) -> String {
    let type_name = "std::vec::Vec";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Expandable array type".into(),
            vec!["T".into()],
            None,
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
            None,
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
            None,
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
            None,
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
            Some(crate::TypeReference::new(
                reflect_type_vector(schema),
                vec!["T".into()],
            )),
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

fn reflect_type_pointer(
    schema: &mut crate::Schema,
    type_name: &str,
    with_lifetime: bool,
) -> String {
    if schema.reserve_type(&type_name) {
        let mut type_def = crate::Primitive::new(
            type_name.into(),
            format!("{type_name} pointer type"),
            vec!["T".into()],
            Some("T".into()),
        );
        if with_lifetime {
            type_def.parameters.insert(0, "'a".into());
        }
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
macro_rules! impl_reflect_pointer {
    ($type:path) => {
        impl<T: Input> Input for $type {
            fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
                crate::TypeReference::new(
                    reflect_type_pointer(schema, &stringify!($type).replace("<T>", ""), false),
                    vec![T::reflect_input_type(schema)],
                )
            }
        }
        impl<T: Output> Output for $type {
            fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
                crate::TypeReference::new(
                    reflect_type_pointer(schema, &stringify!($type).replace("<T>", ""), false),
                    vec![T::reflect_output_type(schema)],
                )
            }
        }
    };
}

impl_reflect_pointer!(std::boxed::Box<T>);
impl_reflect_pointer!(std::rc::Rc<T>);
impl_reflect_pointer!(std::sync::Arc<T>);
impl_reflect_pointer!(std::cell::Cell<T>);
impl_reflect_pointer!(std::cell::RefCell<T>);
impl_reflect_pointer!(std::sync::Mutex<T>);
impl_reflect_pointer!(std::sync::RwLock<T>);
impl_reflect_pointer!(std::sync::Weak<T>);

macro_rules! impl_reflect_pointer_with_lifetime {
    ($type:path) => {
        impl<'a, T: Input> Input for $type {
            fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
                crate::TypeReference::new(
                    reflect_type_pointer(schema, &stringify!($type).replace("<'a, T>", ""), true),
                    vec![T::reflect_input_type(schema)],
                )
            }
        }
        impl<'a, T: Output> Output for $type {
            fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
                crate::TypeReference::new(
                    reflect_type_pointer(schema, &stringify!($type).replace("<'a, T>", ""), true),
                    vec![T::reflect_output_type(schema)],
                )
            }
        }
    };
}
impl_reflect_pointer_with_lifetime!(std::cell::Ref<'a, T>);
impl_reflect_pointer_with_lifetime!(std::cell::RefMut<'a, T>);
impl_reflect_pointer_with_lifetime!(std::sync::MutexGuard<'a, T>);
impl_reflect_pointer_with_lifetime!(std::sync::RwLockReadGuard<'a, T>);
impl_reflect_pointer_with_lifetime!(std::sync::RwLockWriteGuard<'a, T>);

impl<T: Input> Input for *const T {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_pointer(schema, "*const", false),
            vec![T::reflect_input_type(schema)],
        )
    }
}
impl<T: Output> Output for *const T {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_pointer(schema, "*const", false),
            vec![T::reflect_output_type(schema)],
        )
    }
}
impl<T: Input> Input for *mut T {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_pointer(schema, "*mut", false),
            vec![T::reflect_input_type(schema)],
        )
    }
}
impl<T: Output> Output for *mut T {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_pointer(schema, "*mut", false),
            vec![T::reflect_output_type(schema)],
        )
    }
}
impl<'a, T: Input + Clone> Input for std::borrow::Cow<'a, T> {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_pointer(schema, "std::borrow::Cow", true),
            vec![T::reflect_input_type(schema)],
        )
    }
}
impl<'a, T: Output + Clone> Output for std::borrow::Cow<'a, T> {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_pointer(schema, "std::borrow::Cow", true),
            vec![T::reflect_output_type(schema)],
        )
    }
}

fn reflect_type_phantom_data(schema: &mut crate::Schema) -> String {
    let type_name = "std::marker::PhantomData";
    if schema.reserve_type(&type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            format!("Zero-sized phantom data"),
            vec!["T".into()],
            None,
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<T: Input> Input for std::marker::PhantomData<T> {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_phantom_data(schema),
            vec![T::reflect_input_type(schema)],
        )
    }
}
impl<T: Output> Output for std::marker::PhantomData<T> {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        crate::TypeReference::new(
            reflect_type_phantom_data(schema),
            vec![T::reflect_output_type(schema)],
        )
    }
}

impl Input for std::convert::Infallible {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        reflect_type_simple(
            schema,
            "std::convert::Infallible",
            "Never type",
            None,
        )
    }
}
impl Output for std::convert::Infallible {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        reflect_type_simple(
            schema,
            "std::convert::Infallible",
            "Never type",
            None,
        )
    }
}

impl Input for () {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
        reflect_type_simple(schema, "()", "Unit type", None)
    }
}

impl Output for () {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
        reflect_type_simple(schema, "()", "Unit type", None)
    }
}
