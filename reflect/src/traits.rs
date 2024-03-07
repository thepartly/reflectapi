use reflect_schema::TypeReference;

pub trait Input {
    fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference;
}

pub trait Output {
    fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference;
}

macro_rules! impl_reflect {
    ($type:ty, $description:tt) => {
        impl Input for $type {
            fn reflect_input_type(schema: &mut crate::Schema) -> crate::TypeReference {
                let type_name = stringify!($type);
                if schema.reserve_type(type_name) {
                    let type_def =
                        crate::Primitive::new(type_name.into(), $description.into(), Vec::new());
                    schema.insert_type(type_def.into());
                }

                crate::TypeReference::new(type_name.into(), Vec::new())
            }
        }
        impl Output for $type {
            fn reflect_output_type(schema: &mut crate::Schema) -> crate::TypeReference {
                let type_name = stringify!($type);
                if schema.reserve_type(type_name) {
                    let type_def =
                        crate::Primitive::new(type_name.into(), $description.into(), Vec::new());
                    schema.insert_type(type_def.into());
                }
                crate::TypeReference::new(type_name.into(), Vec::new())
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

impl<T: Input> Input for Vec<T> {
    fn reflect_input_type(schema: &mut crate::Schema) -> TypeReference {
        let type_name = "std::vec::Vec";
        if schema.reserve_type(type_name) {
            let type_def = crate::Primitive::new(
                type_name.into(),
                "Expandable array type".into(),
                vec![crate::TypeParameter::new("T".into())],
            );
            schema.insert_type(type_def.into());
        }

        crate::TypeReference::new(type_name.into(), vec![T::reflect_input_type(schema)])
    }
}
