#[derive(serde::Deserialize, reflect::Input)]
struct MyStruct {
    #[serde(unknown_attributes)]
    field: u32,
}

fn main() {}
