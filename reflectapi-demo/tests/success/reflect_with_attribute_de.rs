#[derive(reflectapi::Input)]
struct MyStruct {
    #[reflect(input_type = "u32")]
    field: u32,
}

fn main() {}
