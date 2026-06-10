#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[reflectapi(input_skip)]
    field: u32,
}

fn main() {}
