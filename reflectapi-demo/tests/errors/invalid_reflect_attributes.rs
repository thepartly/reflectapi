#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[reflectapi(unknown_attributes)]
    field: u32,
}

fn main() {}
