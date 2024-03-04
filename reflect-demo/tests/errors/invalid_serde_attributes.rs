#[derive(serde::Deserialize, reflect_derive::Reflect)]
struct MyStruct {
    #[serde(unknown_attributes)]
    field: u32,
}

fn main() {}
