#[derive(serde::Deserialize, reflectapi::Input)]
struct MyStruct {
    #[reflect(output_type)]
    field: u32,
}

fn main() {}
