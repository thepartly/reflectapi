#[derive(serde::Deserialize, reflect::Reflect)]
struct MyStruct {
    #[reflect(serialize_type = "")]
    field: u32,
}

fn main() {}
