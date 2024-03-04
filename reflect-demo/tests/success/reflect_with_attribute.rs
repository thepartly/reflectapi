#[derive(reflect::Input)]
struct MyStruct {
    #[reflect(type = "u32")]
    field: u32,
}

fn main() {}
