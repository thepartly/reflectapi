#[derive(reflect_derive::Reflect)]
struct MyStruct {
    #[reflect(deserialize_type = "u32")]
    field: u32,
}

fn main() {}
