use reflect_schema::Schema;

pub trait Input {
    fn reflect_input_type(schema: &mut Schema) -> String;
}

pub trait Output {
    fn reflect_output_type(schema: &mut Schema) -> String;
}

macro_rules! impl_reflect {
    ($type:ty) => {
        impl Input for $type {
            fn reflect_input_type(schema: &mut Schema) -> String {
                let name = stringify!($type).to_string();
                schema.insert_type(crate::Type::new(name.clone()));
                name
            }
        }
        impl Output for $type {
            fn reflect_output_type(schema: &mut Schema) -> String {
                let name = stringify!($type).to_string();
                schema.insert_type(crate::Type::new(name.clone()));
                name
            }
        }
    };
}

impl_reflect!(i8);
impl_reflect!(i16);
impl_reflect!(i32);
impl_reflect!(i64);
impl_reflect!(u8);
impl_reflect!(u16);
impl_reflect!(u32);
impl_reflect!(u64);
impl_reflect!(f32);
impl_reflect!(f64);
impl_reflect!(bool);
impl_reflect!(char);
impl_reflect!(std::string::String);
