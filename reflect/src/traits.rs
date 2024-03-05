use reflect_schema::Schema;

pub trait Input {
    fn reflect_input() -> crate::Schema;
    fn reflect_input_name() -> String;
    fn reflect_input_type(schema: &mut Schema) -> ();
}

pub trait Output {
    fn reflect_output() -> crate::Schema;
    fn reflect_output_name() -> String;
    fn reflect_output_type(schema: &mut Schema) -> ();
}

pub trait Reflect {
    fn reflect() -> crate::Schema;
    fn reflect_name() -> String;
    fn reflect_type(schema: &mut Schema) -> ();
}

impl<T: Reflect> Input for T {
    fn reflect_input() -> crate::Schema {
        T::reflect()
    }
    fn reflect_input_name() -> String {
        T::reflect_name()
    }
    fn reflect_input_type(schema: &mut Schema) -> () {
        T::reflect_type(schema)
    }
}

impl<T: Reflect> Output for T {
    fn reflect_output() -> crate::Schema {
        T::reflect()
    }
    fn reflect_output_name() -> String {
        T::reflect_name()
    }
    fn reflect_output_type(schema: &mut Schema) -> () {
        T::reflect_type(schema)
    }
}

macro_rules! impl_reflect {
    ($type:ty) => {
        impl Reflect for $type {
            fn reflect() -> crate::Schema {
                let mut schema = Schema::new();
                schema.name = stringify!($type).to_string();
                schema
            }
            fn reflect_name() -> String {
                stringify!($type).to_string()
            }
            fn reflect_type(schema: &mut Schema) -> () {
                schema.insert_type(crate::Type::new(Self::reflect_name()));
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
