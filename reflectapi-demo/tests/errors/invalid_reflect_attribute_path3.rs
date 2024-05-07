#[derive(serde::Deserialize, reflectapi::Output)]
struct MyStruct {
    #[reflectapi(output_type = "invalid")]
    field: u32,
}

fn main() {}
