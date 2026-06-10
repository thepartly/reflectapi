#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[reflectapi(skip)]
    field: u32,
}

fn main() {}
