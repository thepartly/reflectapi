#[derive(reflect::Input)]
struct MyStruct {
    #[reflect(output_type = "u32")]
    field: u32,
}

fn main() {}
