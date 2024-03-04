#[derive(serde::Deserialize, reflect::Input)]
struct MyStruct {
    #[reflect(output_type = "")]
    field: u32,
}

fn main() {}
