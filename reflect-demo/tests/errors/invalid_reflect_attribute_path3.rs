#[derive(serde::Deserialize, reflect_derive::Reflect)]
struct MyStruct {
    #[reflect(serialize_type = "invalid")]
    field: u32,
}

fn main() {}
