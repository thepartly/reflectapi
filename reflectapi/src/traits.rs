/// A trait for types that can be used as an input in an API.
///
/// Implementing this trait allows a type to be introspected and its structure
/// added to a `Typespace` schema. This is essential for automatically generating
/// API documentation, client-side code, and validation rules for function arguments
/// and request bodies.
///
/// ## Derive Macro
///
/// **In most cases, you should not implement this trait manually.** Instead, use the
/// `#[derive(Input)]` macro provided by `reflectapi` to automatically generate
/// a correct and efficient implementation.
///
/// ```rust
/// # use reflectapi::{Input, Output};
///
/// #[derive(Input, Output)]
/// struct User {
///     id: u64,
///     username: String,
///     password_hash: String,
/// }
/// ```
// TODO: We need examples for manual derive and supported flags for the derive macro.
pub trait Input {
    /// Recursively adds the type definition to the schema and returns a reference to it.
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference;
}

/// A trait for types that can be used as an output in an API.
///
/// Implementing this trait allows a type to be introspected and its structure
/// added to a `Typespace` schema. This is essential for automatically generating
/// API documentation, client-side code, and validation rules for function return
/// types and response bodies.
///
/// # Derive Macro
///
/// **In most cases, you should not implement this trait manually.** Instead, use the
/// `#[derive(Output)]` macro provided by `reflectapi` to automatically generate
/// a correct and efficient implementation.
///
/// ```rust
/// # use reflectapi::{Input, Output};
///
/// #[derive(Input, Output)]
/// struct User {
///     id: u64,
///     username: String,
/// }
/// ```
// TODO: We need examples for manual derive and supported flags for the derive macro.
pub trait Output {
    /// Recursively adds the type definition to the schema and returns a reference to it.
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference;
}

pub(crate) fn reflectapi_type_empty(
    schema: &mut crate::Typespace,
    type_name: &str,
    description: &str,
) -> crate::TypeReference {
    if schema.reserve_type(type_name) {
        let mut type_def = crate::Struct::new(type_name);
        type_def.description = description.into();
        schema.insert_type(type_def.into());
    }
    crate::TypeReference::new(type_name, Vec::new())
}

pub(crate) fn reflectapi_type_simple(
    schema: &mut crate::Typespace,
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
    crate::TypeReference::new(type_name, Vec::new())
}

impl Output for &'static str {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_type_simple(schema, "std::string::String", "UTF-8 encoded string", None)
    }
}

macro_rules! impl_reflectapi_simple {
    ($type:ty, $description:tt) => {
        impl Input for $type {
            fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                reflectapi_type_simple(schema, stringify!($type), $description, None)
            }
        }
        impl Output for $type {
            fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                reflectapi_type_simple(schema, stringify!($type), $description, None)
            }
        }
    };
}
impl_reflectapi_simple!(i8, "8-bit signed integer");
impl_reflectapi_simple!(i16, "16-bit signed integer");
impl_reflectapi_simple!(i32, "32-bit signed integer");
impl_reflectapi_simple!(i64, "64-bit signed integer");
impl_reflectapi_simple!(i128, "128-bit signed integer");
impl_reflectapi_simple!(u8, "8-bit unsigned integer");
impl_reflectapi_simple!(u16, "16-bit unsigned integer");
impl_reflectapi_simple!(u32, "32-bit unsigned integer");
impl_reflectapi_simple!(u64, "64-bit unsigned integer");
impl_reflectapi_simple!(u128, "128-bit unsigned integer");
impl_reflectapi_simple!(f32, "32-bit floating point number");
impl_reflectapi_simple!(f64, "64-bit floating point number");
impl_reflectapi_simple!(bool, "Boolean value");
impl_reflectapi_simple!(char, "Unicode character");
impl_reflectapi_simple!(std::string::String, "UTF-8 encoded string");

macro_rules! impl_reflectapi_simple_with_fallback {
    ($type:ty, $description:tt, $fallback:expr) => {
        impl Input for $type {
            fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                reflectapi_type_simple(schema, stringify!($type), $description, $fallback)
            }
        }
        impl Output for $type {
            fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                reflectapi_type_simple(schema, stringify!($type), $description, $fallback)
            }
        }
    };
}

impl_reflectapi_simple_with_fallback!(
    std::num::NonZeroU8,
    "8-bit non-zero unsigned integer",
    Some("u8".into())
);
impl_reflectapi_simple_with_fallback!(
    std::num::NonZeroU16,
    "16-bit non-zero unsigned integer",
    Some("u16".into())
);
impl_reflectapi_simple_with_fallback!(
    std::num::NonZeroU32,
    "32-bit non-zero unsigned integer",
    Some("u32".into())
);
impl_reflectapi_simple_with_fallback!(
    std::num::NonZeroU64,
    "64-bit non-zero unsigned integer",
    Some("u64".into())
);
impl_reflectapi_simple_with_fallback!(
    std::num::NonZeroU128,
    "128-bit non-zero unsigned integer",
    Some("u128".into())
);

impl Input for isize {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        let fallback = Some(i64::reflectapi_input_type(schema));
        reflectapi_type_simple(
            schema,
            "isize",
            "Machine-specific-bit signed integer",
            fallback,
        )
    }
}
impl Output for isize {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        let fallback = Some(i64::reflectapi_output_type(schema));
        reflectapi_type_simple(
            schema,
            "isize",
            "Machine-specific-bit signed integer",
            fallback,
        )
    }
}

impl Input for usize {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        let fallback = Some(u64::reflectapi_input_type(schema));
        reflectapi_type_simple(
            schema,
            "usize",
            "Machine-specific-bit unsigned integer",
            fallback,
        )
    }
}
impl Output for usize {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        let fallback = Some(u64::reflectapi_output_type(schema));
        reflectapi_type_simple(
            schema,
            "usize",
            "Machine-specific-bit unsigned integer",
            fallback,
        )
    }
}

fn reflectapi_type_vector(schema: &mut crate::Typespace) -> String {
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
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_vector(schema),
            vec![T::reflectapi_input_type(schema)],
        )
    }
}
impl<T: Output> Output for Vec<T> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_vector(schema),
            vec![T::reflectapi_output_type(schema)],
        )
    }
}

fn reflectapi_type_option(schema: &mut crate::Typespace) -> String {
    let type_name = "std::option::Option";
    if schema.reserve_type(type_name) {
        let mut type_def = crate::Enum::new(type_name.into());
        type_def.parameters.push("T".into());
        type_def.description = "Optional nullable type".into();
        type_def.representation = crate::Representation::None;

        let mut variant = crate::Variant::new("None".into());
        variant.description = "The value is not provided, i.e. null".into();
        type_def.variants.push(variant);

        let mut variant = crate::Variant::new("Some".into());
        variant.description = "The value is provided and set to some value".into();
        variant.fields = crate::Fields::Unnamed(vec![crate::Field::new("0".into(), "T".into())]);
        type_def.variants.push(variant);

        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<T: Input> Input for Option<T> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_option(schema),
            vec![T::reflectapi_input_type(schema)],
        )
    }
}
impl<T: Output> Output for Option<T> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_option(schema),
            vec![T::reflectapi_output_type(schema)],
        )
    }
}

fn reflectapi_type_btreemap(schema: &mut crate::Typespace) -> String {
    let type_name = "std::collections::BTreeMap";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Ordered key-value map type".into(),
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

impl<K: Input, V: Input> Input for std::collections::BTreeMap<K, V> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_btreemap(schema),
            vec![
                K::reflectapi_input_type(schema),
                V::reflectapi_input_type(schema),
            ],
        )
    }
}

impl<K: Output, V: Output> Output for std::collections::BTreeMap<K, V> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_btreemap(schema),
            vec![
                K::reflectapi_output_type(schema),
                V::reflectapi_output_type(schema),
            ],
        )
    }
}

pub(super) fn reflectapi_type_hashmap(schema: &mut crate::Typespace) -> String {
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
impl<K: Output, V: Output> Output for std::collections::HashMap<K, V> {
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

pub(crate) fn reflectapi_type_hashset(schema: &mut crate::Typespace) -> String {
    let type_name = "std::collections::HashSet";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Value set type".into(),
            vec!["V".into()],
            Some(crate::TypeReference::new(
                reflectapi_type_vector(schema),
                vec!["V".into()],
            )),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<V: Input> Input for std::collections::HashSet<V> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_hashset(schema),
            vec![V::reflectapi_input_type(schema)],
        )
    }
}
impl<V: Output> Output for std::collections::HashSet<V> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_hashset(schema),
            vec![V::reflectapi_output_type(schema)],
        )
    }
}

fn reflectapi_type_btreeset(schema: &mut crate::Typespace) -> String {
    let type_name = "std::collections::BTreeSet";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Ordered set type".into(),
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

impl<V: Input> Input for std::collections::BTreeSet<V> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_btreeset(schema),
            vec![V::reflectapi_input_type(schema)],
        )
    }
}

impl<V: Output> Output for std::collections::BTreeSet<V> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_btreeset(schema),
            vec![V::reflectapi_output_type(schema)],
        )
    }
}

fn reflectapi_type_tuple(schema: &mut crate::Typespace, count: usize) -> String {
    let type_name = format!("std::tuple::Tuple{count}");
    if schema.reserve_type(&type_name) {
        let parameters = (1..(count + 1)).map(|i| format!("T{i}").into()).collect();
        let type_def = crate::Primitive::new(
            type_name.clone(),
            format!("Tuple holding {count} elements"),
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
macro_rules! impl_reflectapi_tuple {
    ( $( $name:ident )+)  => {
        impl<$($name: Input),+> Input for ($($name,)+)
        {
            fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                let type_name = reflectapi_type_tuple(schema, count!($($name)*));
                crate::TypeReference::new(
                    type_name,
                    vec![$($name::reflectapi_input_type(schema)),+],
                )
            }
        }

        impl<$($name: Output),+> Output for ($($name,)+)
        {
            fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                let type_name = reflectapi_type_tuple(schema, count!($($name)*));
                crate::TypeReference::new(
                    type_name,
                    vec![$($name::reflectapi_output_type(schema)),+],
                )
            }
        }
    };
}

impl_reflectapi_tuple! { A }
impl_reflectapi_tuple! { A B }
impl_reflectapi_tuple! { A B C }
impl_reflectapi_tuple! { A B C D }
impl_reflectapi_tuple! { A B C D E }
impl_reflectapi_tuple! { A B C D E F }
impl_reflectapi_tuple! { A B C D E F G }
impl_reflectapi_tuple! { A B C D E F G H }
impl_reflectapi_tuple! { A B C D E F G H I }
impl_reflectapi_tuple! { A B C D E F G H I J }
impl_reflectapi_tuple! { A B C D E F G H I J K }
impl_reflectapi_tuple! { A B C D E F G H I J K L }

fn reflectapi_type_array(schema: &mut crate::Typespace) -> String {
    let type_name = "std::array::Array";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Fixed-size Array".to_string(),
            vec!["T".into(), "N".to_string().into()],
            Some(crate::TypeReference::new(
                reflectapi_type_vector(schema),
                vec!["T".into()],
            )),
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<T: Input, const N: usize> Input for [T; N] {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_array(schema),
            vec![T::reflectapi_input_type(schema), N.to_string().into()],
        )
    }
}
impl<T: Output, const N: usize> Output for [T; N] {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_array(schema),
            vec![T::reflectapi_output_type(schema), N.to_string().into()],
        )
    }
}

fn reflectapi_type_pointer(
    schema: &mut crate::Typespace,
    type_name: &str,
    with_lifetime: bool,
) -> String {
    if schema.reserve_type(type_name) {
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
macro_rules! impl_reflectapi_pointer {
    ($type:path) => {
        impl<T: Input> Input for $type {
            fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                crate::TypeReference::new(
                    reflectapi_type_pointer(schema, &stringify!($type).replace("<T>", ""), false),
                    vec![T::reflectapi_input_type(schema)],
                )
            }
        }
        impl<T: Output> Output for $type {
            fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                crate::TypeReference::new(
                    reflectapi_type_pointer(schema, &stringify!($type).replace("<T>", ""), false),
                    vec![T::reflectapi_output_type(schema)],
                )
            }
        }
    };
}

impl_reflectapi_pointer!(std::boxed::Box<T>);
impl_reflectapi_pointer!(std::rc::Rc<T>);
impl_reflectapi_pointer!(std::sync::Arc<T>);
impl_reflectapi_pointer!(std::cell::Cell<T>);
impl_reflectapi_pointer!(std::cell::RefCell<T>);
impl_reflectapi_pointer!(std::sync::Mutex<T>);
impl_reflectapi_pointer!(std::sync::RwLock<T>);
impl_reflectapi_pointer!(std::sync::Weak<T>);

macro_rules! impl_reflectapi_pointer_with_lifetime {
    ($type:path) => {
        impl<'a, T: Input> Input for $type {
            fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                crate::TypeReference::new(
                    reflectapi_type_pointer(
                        schema,
                        &stringify!($type).replace("<'a, T>", ""),
                        true,
                    ),
                    vec![T::reflectapi_input_type(schema)],
                )
            }
        }
        impl<'a, T: Output> Output for $type {
            fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
                crate::TypeReference::new(
                    reflectapi_type_pointer(
                        schema,
                        &stringify!($type).replace("<'a, T>", ""),
                        true,
                    ),
                    vec![T::reflectapi_output_type(schema)],
                )
            }
        }
    };
}
impl_reflectapi_pointer_with_lifetime!(std::cell::Ref<'a, T>);
impl_reflectapi_pointer_with_lifetime!(std::cell::RefMut<'a, T>);
impl_reflectapi_pointer_with_lifetime!(std::sync::MutexGuard<'a, T>);
impl_reflectapi_pointer_with_lifetime!(std::sync::RwLockReadGuard<'a, T>);
impl_reflectapi_pointer_with_lifetime!(std::sync::RwLockWriteGuard<'a, T>);

impl<T: Input> Input for *const T {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_pointer(schema, "*const", false),
            vec![T::reflectapi_input_type(schema)],
        )
    }
}
impl<T: Output> Output for *const T {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_pointer(schema, "*const", false),
            vec![T::reflectapi_output_type(schema)],
        )
    }
}
impl<T: Input> Input for *mut T {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_pointer(schema, "*mut", false),
            vec![T::reflectapi_input_type(schema)],
        )
    }
}
impl<T: Output> Output for *mut T {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_pointer(schema, "*mut", false),
            vec![T::reflectapi_output_type(schema)],
        )
    }
}
impl<T: Input + Clone> Input for std::borrow::Cow<'_, T> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_pointer(schema, "std::borrow::Cow", true),
            vec![T::reflectapi_input_type(schema)],
        )
    }
}
impl<T: Output + Clone> Output for std::borrow::Cow<'_, T> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_pointer(schema, "std::borrow::Cow", true),
            vec![T::reflectapi_output_type(schema)],
        )
    }
}

fn reflectapi_type_phantom_data(schema: &mut crate::Typespace) -> String {
    let type_name = "std::marker::PhantomData";
    if schema.reserve_type(type_name) {
        let type_def = crate::Primitive::new(
            type_name.into(),
            "Zero-sized phantom data".to_string(),
            vec!["T".into()],
            None,
        );
        schema.insert_type(type_def.into());
    }
    type_name.into()
}
impl<T: Input> Input for std::marker::PhantomData<T> {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_phantom_data(schema),
            vec![T::reflectapi_input_type(schema)],
        )
    }
}
impl<T: Output> Output for std::marker::PhantomData<T> {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        crate::TypeReference::new(
            reflectapi_type_phantom_data(schema),
            vec![T::reflectapi_output_type(schema)],
        )
    }
}

impl Input for std::convert::Infallible {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        // schema builder handles Infallible in a special way
        crate::infallible::Infallible::reflectapi_input_type(schema)
    }
}
impl Output for std::convert::Infallible {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        // schema builder handles Infallible in a special way
        crate::infallible::Infallible::reflectapi_output_type(schema)
    }
}

impl Input for () {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_type_simple(schema, "std::tuple::Tuple0", "Unit type", None)
    }
}

impl Output for () {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_type_simple(schema, "std::tuple::Tuple0", "Unit type", None)
    }
}

fn reflectapi_duration(schema: &mut crate::Typespace) -> crate::TypeReference {
    let type_name = "std::time::Duration";
    if schema.reserve_type(type_name) {
        let type_def = crate::Struct {
            name: type_name.into(),
            description: "Time duration type".into(),
            fields: crate::Fields::Named(vec![
                crate::Field::new("secs".into(), "u64".into()).with_required(true),
                crate::Field::new("nanos".into(), "u32".into()).with_required(true),
            ]),
            serde_name: Default::default(),
            parameters: Default::default(),
            transparent: Default::default(),
            codegen_config: Default::default(),
        };

        schema.insert_type(type_def.into());
    }

    crate::TypeReference::new(type_name, Vec::new())
}
impl Input for std::time::Duration {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        u64::reflectapi_input_type(schema);
        u32::reflectapi_input_type(schema);
        reflectapi_duration(schema)
    }
}
impl Output for std::time::Duration {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        u64::reflectapi_output_type(schema);
        u32::reflectapi_output_type(schema);
        reflectapi_duration(schema)
    }
}

impl Input for std::path::PathBuf {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_type_simple(
            schema,
            "std::path::PathBuf",
            "File path type",
            Some("std::string::String".into()),
        )
    }
}
impl Output for std::path::PathBuf {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_type_simple(
            schema,
            "std::path::PathBuf",
            "File path type",
            Some("std::string::String".into()),
        )
    }
}

impl Input for std::path::Path {
    fn reflectapi_input_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_type_simple(
            schema,
            "std::path::Path",
            "File path type",
            Some("std::path::PathBuf".into()),
        )
    }
}
impl Output for std::path::Path {
    fn reflectapi_output_type(schema: &mut crate::Typespace) -> crate::TypeReference {
        reflectapi_type_simple(
            schema,
            "std::path::Path",
            "File path type",
            Some("std::path::PathBuf".into()),
        )
    }
}
