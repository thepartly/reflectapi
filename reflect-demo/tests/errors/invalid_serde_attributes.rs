#[derive(serde::Deserialize, reflect::Reflect)]
struct MyStruct {
    #[serde(unknown_attributes)]
    field: u32,
}

fn main() {}
