#[derive(serde::Deserialize, reflect::Reflect)]
struct MyStruct {
    #[reflect(serialize_type = "invalid")]
    field: u32,
}

fn main() {}
