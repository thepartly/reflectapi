#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[reflect(unknown_attributes)]
    field: u32,
}

fn main() {}
