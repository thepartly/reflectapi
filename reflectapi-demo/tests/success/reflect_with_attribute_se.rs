#[derive(reflectapi::Input)]
struct MyStruct {
    #[reflectapi(output_type = "u32")]
    field: u32,
}

fn main() {}
