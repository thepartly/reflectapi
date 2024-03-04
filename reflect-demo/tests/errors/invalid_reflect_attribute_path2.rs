#[derive(serde::Deserialize, reflect_derive::Reflect)]
struct MyStruct {
    #[reflect(serialize_type = "")]
    field: u32,
}

fn main() {}
