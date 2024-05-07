#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[reflectapi(output_type)]
    field: u32,
}

fn main() {}
