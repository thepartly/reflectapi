#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[serde(unknown_attributes)]
    field: u32,
}

fn main() {}
