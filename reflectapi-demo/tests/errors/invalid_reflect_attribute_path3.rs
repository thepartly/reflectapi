#[derive(serde::Deserialize, reflectapi::Output)]
struct MyStruct {
    #[reflect(output_type = "invalid")]
    field: u32,
}

fn main() {}
