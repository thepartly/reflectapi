#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[reflectapi(hidden)]
    field: u32,
}

fn main() {}
