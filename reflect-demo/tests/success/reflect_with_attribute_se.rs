#[derive(reflect::Reflect)]
struct MyStruct {
    #[reflect(serialize_type = "u32")]
    field: u32,
}

fn main() {}
