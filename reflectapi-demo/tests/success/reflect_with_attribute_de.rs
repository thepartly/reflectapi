#[derive(reflectapi::Input)]
struct MyStruct {
    #[reflectapi(input_type = "u32")]
    field: u32,
}

fn main() {}
